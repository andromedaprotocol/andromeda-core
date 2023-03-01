use crate::state::{
    CONDITION_ADO_ADDRESS, OPERATION, ORACLE_ADO_ADDRESS, TASK_BALANCER_ADDRESS, VALUE,
};
use ado_base::state::ADOContract;
use andromeda_automation::evaluation::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, Operators, QueryMsg,
};
use andromeda_automation::oracle::QueryMsg as OracleQueryMsg;
use andromeda_automation::task_balancer::ExecuteMsg::Remove;
use common::app::GetAddress;
use common::{ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError};
use cosmwasm_std::{
    ensure, entry_point, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    QueryRequest, Reply, Response, StdError, SubMsg, Uint128, WasmMsg, WasmQuery,
};
use cw2::{get_contract_version, set_contract_version};
use cw_utils::nonpayable;

use semver::Version;
use std::env;
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-evaluation";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONDITION_ADO_ADDRESS.save(deps.storage, &msg.condition_address)?;
    ORACLE_ADO_ADDRESS.save(deps.storage, &msg.oracle_address)?;
    TASK_BALANCER_ADDRESS.save(deps.storage, &msg.task_balancer)?;

    // If the user doesn't provide a value, we assume that the oracle ADO will be returning a boolean
    if let Some(user_value) = msg.user_value {
        VALUE.save(deps.storage, &Some(user_value))?;
    } else {
        VALUE.save(deps.storage, &None)?;
    }

    OPERATION.save(deps.storage, &msg.operation)?;

    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "evaluation".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: None,
            kernel_address: msg.kernel_address,
        },
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    // Load task balancer's address
    let contract_addr = TASK_BALANCER_ADDRESS.load(deps.storage)?;
    let app_contract = ADOContract::default().get_app_contract(deps.storage)?;
    let contract_addr = contract_addr.get_address(deps.api, &deps.querier, app_contract.clone())?;
    if let Some(app_address) = app_contract {
        match reply.id {
            // this represents the id of the Execute error which requires removal of the entire process
            1 => Ok(Response::new().add_submessage(SubMsg::new(CosmosMsg::Wasm(
                WasmMsg::Execute {
                    contract_addr,
                    msg: to_binary(&Remove {
                        process: app_address.into_string(),
                    })?,
                    funds: vec![],
                },
            )))),
            _ => Err(ContractError::AccountNotFound {}),
        }
    } else {
        Err(ContractError::AccountNotFound {})
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    match msg {
        ExecuteMsg::AndrReceive(msg) => contract.execute(deps, env, info, msg, execute),
        ExecuteMsg::ChangeConditionAddress { address } => {
            execute_change_condition_address(deps, env, info, address)
        }
        ExecuteMsg::ChangeQueryAddress { address } => {
            execute_change_query_address(deps, env, info, address)
        }
    }
}

fn execute_change_query_address(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    // Only the contract's owner can update the Execute ADO address
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    ORACLE_ADO_ADDRESS.save(deps.storage, &address)?;
    Ok(Response::new()
        .add_attribute("action", "changed_ORACLE_ADO_ADDRESS")
        .add_attribute("new_address", address))
}

fn execute_change_condition_address(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    // Only the contract's owner can update the Execute ADO address
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    CONDITION_ADO_ADDRESS.save(deps.storage, &address)?;
    Ok(Response::new()
        .add_attribute("action", "changed_CONDITION_ADO_ADDRESS")
        .add_attribute("new_address", address))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // New version
    let version: Version = CONTRACT_VERSION.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    let contract = ADOContract::default();

    ensure!(
        stored.contract == CONTRACT_NAME,
        ContractError::CannotMigrate {
            previous_contract: stored.contract,
        }
    );

    // New version has to be newer/greater than the old version
    ensure!(
        storage_version < version,
        ContractError::CannotMigrate {
            previous_contract: stored.version,
        }
    );

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Update the ADOContract's version
    contract.execute_update_version(deps)?;

    Ok(Response::default())
}

fn from_semver(err: semver::Error) -> StdError {
    StdError::generic_err(format!("Semver: {err}"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::ConditionADO {} => encode_binary(&query_condition_ado(deps)?),
        QueryMsg::Evaluation {} => encode_binary(&query_evaluation(deps, env)?),
        QueryMsg::OracleADO {} => encode_binary(&query_oracle_ado(deps)?),
    }
}

fn query_evaluation(deps: Deps, _env: Env) -> Result<bool, ContractError> {
    let contract = ADOContract::default();
    let app_contract = contract.get_app_contract(deps.storage)?;

    let operation = OPERATION.load(deps.storage)?;
    let user_value = VALUE.load(deps.storage)?;

    // Get the address of the oracle contract that will provide data to be compared with the user's data
    let oracle_addr = ORACLE_ADO_ADDRESS.load(deps.storage)?.get_address(
        deps.api,
        &deps.querier,
        app_contract,
    )?;

    let result = if let Some(user_value) = user_value {
        let oracle_value: String = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: oracle_addr,
            msg: to_binary(&OracleQueryMsg::Target {})?,
        }))?;

        // In the future, user will set the expected value during instantiation and parse it accordingly
        let parsed_oracle_value: Uint128 = oracle_value.parse()?;

        match operation {
            Operators::Greater => parsed_oracle_value > user_value,
            Operators::GreaterEqual => parsed_oracle_value >= user_value,
            Operators::Equal => parsed_oracle_value == user_value,
            Operators::LessEqual => parsed_oracle_value <= user_value,
            Operators::Less => parsed_oracle_value < user_value,
        }
        // If the user didn't provide a value, we assume the query ADO returns a bool
    } else {
        let oracle_value: String = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: oracle_addr,
            msg: to_binary(&OracleQueryMsg::Target {})?,
        }))?;

        let parsed_oracle_value: bool = oracle_value.parse()?;

        parsed_oracle_value
    };

    Ok(result)
}

fn query_oracle_ado(deps: Deps) -> Result<String, ContractError> {
    let address = ORACLE_ADO_ADDRESS.load(deps.storage)?;
    Ok(address)
}

fn query_condition_ado(deps: Deps) -> Result<String, ContractError> {
    let address = CONDITION_ADO_ADDRESS.load(deps.storage)?;
    Ok(address)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_querier::{mock_dependencies_custom, MOCK_QUERY_CONTRACT};
    use andromeda_automation::evaluation::Operators;
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::{mock_env, mock_info};

    #[test]
    fn test_initialization() {
        let mut deps = mock_dependencies_custom(&[]);

        let condition_address = "condition_address".to_string();
        let oracle_address = MOCK_QUERY_CONTRACT.to_string();
        let task_balancer = "task_balancer_address".to_string();

        let user_value = Some(Uint128::from(10u32));
        let operation = Operators::Greater;
        let msg = InstantiateMsg {
            condition_address,
            oracle_address,
            task_balancer,
            user_value,
            operation,
            kernel_address: None,
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // make sure address was saved correctly
        let addr = CONDITION_ADO_ADDRESS.load(&deps.storage).unwrap();
        assert_eq!(addr, "condition_address".to_string());

        let addr = ORACLE_ADO_ADDRESS.load(&deps.storage).unwrap();
        assert_eq!(addr, MOCK_QUERY_CONTRACT.to_string())
    }

    #[test]
    fn test_evaluate_greater_is_greater() {
        let mut deps = mock_dependencies_custom(&[]);
        let condition_address = "condition_address".to_string();
        let oracle_address = MOCK_QUERY_CONTRACT.to_string();
        let task_balancer = "task_balancer_address".to_string();
        let operation = Operators::Greater;
        let user_value = Some(Uint128::from(30u32));
        let msg = InstantiateMsg {
            condition_address,
            oracle_address,
            task_balancer,
            user_value,
            operation,
            kernel_address: None,
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        let msg = QueryMsg::Evaluation {};

        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let expected = true;
        assert_eq!(from_binary::<bool>(&res).unwrap(), expected);
    }

    #[test]
    fn test_evaluate_greater_is_less() {
        let mut deps = mock_dependencies_custom(&[]);

        let condition_address = "condition_address".to_string();
        let oracle_address = MOCK_QUERY_CONTRACT.to_string();
        let task_balancer = "task_balancer_address".to_string();
        let operation = Operators::Greater;
        let user_value = Some(Uint128::from(130u32));
        let msg = InstantiateMsg {
            condition_address,
            oracle_address,
            task_balancer,
            user_value,
            operation,
            kernel_address: None,
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = QueryMsg::Evaluation {};

        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let expected = false;
        assert_eq!(from_binary::<bool>(&res).unwrap(), expected);
    }

    #[test]
    fn test_evaluate_greater_is_equal() {
        let mut deps = mock_dependencies_custom(&[]);

        let condition_address = "condition_address".to_string();
        let oracle_address = MOCK_QUERY_CONTRACT.to_string();
        let task_balancer = "task_balancer_address".to_string();
        let operation = Operators::Greater;
        let user_value = Some(Uint128::from(40u32));
        let msg = InstantiateMsg {
            condition_address,
            oracle_address,
            task_balancer,
            user_value,
            operation,
            kernel_address: None,
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = QueryMsg::Evaluation {};

        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let expected = false;
        assert_eq!(from_binary::<bool>(&res).unwrap(), expected);
    }

    #[test]
    fn test_evaluate_greater_equal_is_equal() {
        let mut deps = mock_dependencies_custom(&[]);

        let operation = Operators::GreaterEqual;
        let condition_address = "condition_address".to_string();
        let oracle_address = MOCK_QUERY_CONTRACT.to_string();
        let task_balancer = "task_balancer_address".to_string();
        let user_value = Some(Uint128::from(40u32));
        let msg = InstantiateMsg {
            condition_address,
            oracle_address,
            task_balancer,
            user_value,
            operation,
            kernel_address: None,
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = QueryMsg::Evaluation {};

        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let expected = true;
        assert_eq!(from_binary::<bool>(&res).unwrap(), expected);
    }

    #[test]
    fn test_evaluate_greater_equal_is_greater() {
        let mut deps = mock_dependencies_custom(&[]);

        let operation = Operators::GreaterEqual;
        let condition_address = "condition_address".to_string();
        let oracle_address = MOCK_QUERY_CONTRACT.to_string();
        let task_balancer = "task_balancer_address".to_string();
        let user_value = Some(Uint128::from(30u32));
        let msg = InstantiateMsg {
            condition_address,
            oracle_address,
            task_balancer,
            user_value,
            operation,
            kernel_address: None,
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = QueryMsg::Evaluation {};

        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let expected = true;
        assert_eq!(from_binary::<bool>(&res).unwrap(), expected);
    }

    #[test]
    fn test_evaluate_greater_equal_is_less() {
        let mut deps = mock_dependencies_custom(&[]);

        let operation = Operators::GreaterEqual;
        let condition_address = "condition_address".to_string();
        let oracle_address = MOCK_QUERY_CONTRACT.to_string();
        let task_balancer = "task_balancer_address".to_string();
        let user_value = Some(Uint128::from(140u32));
        let msg = InstantiateMsg {
            condition_address,
            oracle_address,
            task_balancer,
            user_value,
            operation,
            kernel_address: None,
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = QueryMsg::Evaluation {};

        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let expected = false;
        assert_eq!(from_binary::<bool>(&res).unwrap(), expected);
    }

    #[test]
    fn test_evaluate_equal_is_equal() {
        let mut deps = mock_dependencies_custom(&[]);

        let operation = Operators::Equal;
        let condition_address = "condition_address".to_string();
        let oracle_address = MOCK_QUERY_CONTRACT.to_string();
        let task_balancer = "task_balancer_address".to_string();
        let user_value = Some(Uint128::from(40u32));
        let msg = InstantiateMsg {
            condition_address,
            oracle_address,
            task_balancer,
            user_value,
            operation,
            kernel_address: None,
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = QueryMsg::Evaluation {};
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();

        let expected = true;
        assert_eq!(from_binary::<bool>(&res).unwrap(), expected);
    }

    #[test]
    fn test_evaluate_equal_is_greater() {
        let mut deps = mock_dependencies_custom(&[]);

        let operation = Operators::Equal;
        let condition_address = "condition_address".to_string();
        let oracle_address = MOCK_QUERY_CONTRACT.to_string();
        let task_balancer = "task_balancer_address".to_string();
        let user_value = Some(Uint128::from(30u32));
        let msg = InstantiateMsg {
            condition_address,
            oracle_address,
            task_balancer,
            user_value,
            operation,
            kernel_address: None,
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = QueryMsg::Evaluation {};
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();

        let expected = false;
        assert_eq!(from_binary::<bool>(&res).unwrap(), expected);
    }

    #[test]
    fn test_evaluate_equal_is_less() {
        let mut deps = mock_dependencies_custom(&[]);

        let operation = Operators::Equal;
        let condition_address = "condition_address".to_string();
        let oracle_address = MOCK_QUERY_CONTRACT.to_string();
        let task_balancer = "task_balancer_address".to_string();
        let user_value = Some(Uint128::from(1140u32));
        let msg = InstantiateMsg {
            condition_address,
            oracle_address,
            task_balancer,
            user_value,
            operation,
            kernel_address: None,
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = QueryMsg::Evaluation {};
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();

        let expected = false;
        assert_eq!(from_binary::<bool>(&res).unwrap(), expected);
    }

    #[test]
    fn test_evaluate_less_equal_is_less() {
        let mut deps = mock_dependencies_custom(&[]);

        let operation = Operators::LessEqual;
        let condition_address = "condition_address".to_string();
        let oracle_address = MOCK_QUERY_CONTRACT.to_string();
        let task_balancer = "task_balancer_address".to_string();
        let user_value = Some(Uint128::from(1140u32));
        let msg = InstantiateMsg {
            condition_address,
            oracle_address,
            task_balancer,
            user_value,
            operation,
            kernel_address: None,
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = QueryMsg::Evaluation {};
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();

        let expected = true;
        assert_eq!(from_binary::<bool>(&res).unwrap(), expected);
    }

    #[test]
    fn test_evaluate_less_equal_is_equal() {
        let mut deps = mock_dependencies_custom(&[]);

        let operation = Operators::LessEqual;
        let condition_address = "condition_address".to_string();
        let oracle_address = MOCK_QUERY_CONTRACT.to_string();
        let task_balancer = "task_balancer_address".to_string();
        let user_value = Some(Uint128::from(40u32));
        let msg = InstantiateMsg {
            condition_address,
            oracle_address,
            task_balancer,
            user_value,
            operation,
            kernel_address: None,
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = QueryMsg::Evaluation {};

        let res = query(deps.as_ref(), mock_env(), msg).unwrap();

        let expected = true;
        assert_eq!(from_binary::<bool>(&res).unwrap(), expected);
    }

    #[test]
    fn test_evaluate_less_equal_is_greater() {
        let mut deps = mock_dependencies_custom(&[]);

        let operation = Operators::LessEqual;
        let condition_address = "condition_address".to_string();
        let oracle_address = MOCK_QUERY_CONTRACT.to_string();
        let task_balancer = "task_balancer_address".to_string();
        let user_value = Some(Uint128::from(30u32));
        let msg = InstantiateMsg {
            condition_address,
            oracle_address,
            task_balancer,
            user_value,
            operation,
            kernel_address: None,
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = QueryMsg::Evaluation {};

        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let expected = false;
        assert_eq!(from_binary::<bool>(&res).unwrap(), expected);
    }

    #[test]
    fn test_evaluate_less_is_greater() {
        let mut deps = mock_dependencies_custom(&[]);

        let operation = Operators::Less;
        let condition_address = "condition_address".to_string();
        let oracle_address = MOCK_QUERY_CONTRACT.to_string();
        let task_balancer = "task_balancer_address".to_string();
        let user_value = Some(Uint128::from(30u32));
        let msg = InstantiateMsg {
            condition_address,
            oracle_address,
            task_balancer,
            user_value,
            operation,
            kernel_address: None,
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = QueryMsg::Evaluation {};

        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let expected = false;
        assert_eq!(from_binary::<bool>(&res).unwrap(), expected);
    }

    #[test]
    fn test_evaluate_less_is_equal() {
        let mut deps = mock_dependencies_custom(&[]);

        let operation = Operators::Less;
        let condition_address = "condition_address".to_string();
        let oracle_address = MOCK_QUERY_CONTRACT.to_string();
        let task_balancer = "task_balancer_address".to_string();
        let user_value = Some(Uint128::from(40u32));
        let msg = InstantiateMsg {
            condition_address,
            oracle_address,
            task_balancer,
            user_value,
            operation,
            kernel_address: None,
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = QueryMsg::Evaluation {};

        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let expected = false;
        assert_eq!(from_binary::<bool>(&res).unwrap(), expected);
    }

    #[test]
    fn test_evaluate_less_is_less() {
        let mut deps = mock_dependencies_custom(&[]);

        let operation = Operators::Less;
        let condition_address = "condition_address".to_string();
        let oracle_address = MOCK_QUERY_CONTRACT.to_string();

        let task_balancer = "task_balancer_address".to_string();

        let user_value = Some(Uint128::from(140u32));
        let msg = InstantiateMsg {
            condition_address,
            oracle_address,
            task_balancer,
            user_value,
            operation,
            kernel_address: None,
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = QueryMsg::Evaluation {};

        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let expected = true;
        assert_eq!(from_binary::<bool>(&res).unwrap(), expected);
    }

    #[test]
    fn test_change_address_unauthorized() {
        let mut deps = mock_dependencies_custom(&[]);

        let condition_address = "condition_address".to_string();
        let oracle_address = MOCK_QUERY_CONTRACT.to_string();

        let task_balancer = "task_balancer_address".to_string();
        let operation = Operators::Less;

        let user_value = Some(Uint128::from(30u32));
        let msg = InstantiateMsg {
            condition_address,
            oracle_address,
            task_balancer,
            user_value,
            operation,
            kernel_address: None,
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let address = "new_address".to_string();
        let msg = ExecuteMsg::ChangeConditionAddress { address };
        let info = mock_info("random", &[]);

        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {})
    }

    #[test]
    fn test_change_address_works() {
        let mut deps = mock_dependencies_custom(&[]);

        let condition_address = "condition_address".to_string();
        let oracle_address = MOCK_QUERY_CONTRACT.to_string();

        let task_balancer = "task_balancer_address".to_string();

        let operation = Operators::Less;

        let user_value = Some(Uint128::from(30u32));
        let msg = InstantiateMsg {
            condition_address,
            oracle_address,
            task_balancer,
            user_value,
            operation,
            kernel_address: None,
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let address = "new_address".to_string();
        let msg = ExecuteMsg::ChangeConditionAddress {
            address: address.clone(),
        };

        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let actual = CONDITION_ADO_ADDRESS.load(&deps.storage).unwrap();
        assert_eq!(address, actual)
    }
}
