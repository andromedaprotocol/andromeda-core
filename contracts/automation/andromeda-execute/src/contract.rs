use std::env;

use crate::state::{
    CONDITION_ADO_ADDRESS, INCREMENT_MESSAGE, TARGET_ADO_ADDRESS, TARGET_MSG, TASK_BALANCER,
};
use ado_base::state::ADOContract;
use andromeda_automation::execute::{ExecuteMsg, Increment, InstantiateMsg, MigrateMsg, QueryMsg};
use andromeda_automation::task_balancer::ExecuteMsg as TaskBalancerExecuteMsg;
use common::{
    ado_base::InstantiateMsg as BaseInstantiateMsg, app::AndrAddress, encode_binary,
    error::ContractError,
};

use cosmwasm_std::{
    ensure, entry_point, from_binary, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Reply, Response, StdError, SubMsg, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw_utils::nonpayable;
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-execute";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    TARGET_ADO_ADDRESS.save(deps.storage, &msg.target_address)?;
    CONDITION_ADO_ADDRESS.save(deps.storage, &msg.condition_address)?;
    INCREMENT_MESSAGE.save(deps.storage, &msg.increment)?;
    TARGET_MSG.save(deps.storage, &msg.target_message)?;
    // Validate task balancer address
    let task_balancer = deps.api.addr_validate(&msg.task_balancer)?;
    TASK_BALANCER.save(deps.storage, &task_balancer)?;

    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "execute".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: None,
            primitive_contract: None,
        },
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let app_contract = contract.get_app_contract(deps.storage)?;
    // Execute errors warrant the removal of the process from the storage contract
    if msg.id == 1 {
        Ok(Response::new().add_submessage(SubMsg::reply_on_error(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: TASK_BALANCER.load(deps.storage)?.to_string(),
                msg: to_binary(&TaskBalancerExecuteMsg::Remove {
                    process: app_contract.unwrap().to_string(),
                })?,
                funds: vec![],
            }),
            1,
        )))
    } else if msg.result.is_err() {
        Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )))
    } else {
        Ok(Response::default())
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
        ExecuteMsg::Execute {} => execute_target(deps, env, info),
        ExecuteMsg::UpdateConditionAddress { condition_address } => {
            execute_update_condition_address(deps, env, info, condition_address)
        }
    }
}

fn execute_update_condition_address(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    condition_address: AndrAddress,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    // Only the contract's owner can update the Execute ADO address
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    CONDITION_ADO_ADDRESS.save(deps.storage, &condition_address)?;

    Ok(Response::new().add_attribute("action", "updated_condition_address"))
}

fn execute_target(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let app_contract = contract.get_app_contract(deps.storage)?;

    let condition_ado = CONDITION_ADO_ADDRESS.load(deps.storage)?.get_address(
        deps.api,
        &deps.querier,
        app_contract.clone(),
    )?;

    ensure!(info.sender == condition_ado, ContractError::Unauthorized {});

    // Target contract's address
    let contract_addr = TARGET_ADO_ADDRESS.load(deps.storage)?.get_address(
        deps.api,
        &deps.querier,
        app_contract,
    )?;

    // Load the stored Target Message
    let stored_msg = TARGET_MSG.load(deps.storage)?;
    let msg: Binary = from_binary(&stored_msg)?;

    let increment = INCREMENT_MESSAGE.load(deps.storage)?;
    match increment {
        Increment::One => Ok(Response::new().add_submessage(SubMsg::reply_on_error(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&msg)?,
                funds: vec![],
            }),
            1,
        ))),
        Increment::Two => Ok(Response::new().add_submessage(SubMsg::reply_on_error(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&msg)?,
                funds: vec![],
            }),
            1,
        ))),
    }
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
    StdError::generic_err(format!("Semver: {}", err))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::TargetADO {} => encode_binary(&query_execute_ado(deps)?),
        QueryMsg::ConditionADO {} => encode_binary(&query_condition_ado(deps)?),
    }
}

fn query_condition_ado(deps: Deps) -> Result<String, ContractError> {
    let address = CONDITION_ADO_ADDRESS.load(deps.storage)?;
    Ok(address.identifier)
}

fn query_execute_ado(deps: Deps) -> Result<String, ContractError> {
    let address = TARGET_ADO_ADDRESS.load(deps.storage)?;
    Ok(address.identifier)
}

#[cfg(test)]
mod tests {
    use super::*;
    use andromeda_automation::counter;
    use andromeda_automation::execute::Increment;
    use common::app::AndrAddress;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn test_initialization() {
        let mut deps = mock_dependencies();
        let target_address = AndrAddress {
            identifier: "target_address".to_string(),
        };
        let condition_address = AndrAddress {
            identifier: "condition_address".to_string(),
        };

        let msg = InstantiateMsg {
            target_address,
            condition_address,
            increment: Increment::One,
            task_balancer: "task_balancer".to_string(),
            target_message: to_binary(&"something").unwrap(),
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // make sure address was saved correctly
        let addr = TARGET_ADO_ADDRESS.load(&deps.storage).unwrap();
        assert_eq!(
            addr,
            AndrAddress {
                identifier: "target_address".to_string(),
            }
        )
    }

    #[test]
    fn test_execute_unauthorized() {
        let mut deps = mock_dependencies();
        let target_address = AndrAddress {
            identifier: "target_address".to_string(),
        };
        let condition_address = AndrAddress {
            identifier: "condition_address".to_string(),
        };

        let msg = InstantiateMsg {
            target_address,
            condition_address,
            increment: Increment::One,
            task_balancer: "task_balancer".to_string(),
            target_message: to_binary(&"something").unwrap(),
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // make sure address was saved correctly
        let addr = TARGET_ADO_ADDRESS.load(&deps.storage).unwrap();
        assert_eq!(
            addr,
            AndrAddress {
                identifier: "target_address".to_string(),
            }
        );

        let msg = ExecuteMsg::Execute {};
        let info = mock_info("not_condition_address", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {})
    }

    #[test]
    fn test_execute() {
        let mut deps = mock_dependencies();
        let target_address = AndrAddress {
            identifier: "target_address".to_string(),
        };
        let condition_address = AndrAddress {
            identifier: "condition_address".to_string(),
        };

        let msg = InstantiateMsg {
            target_address,
            condition_address,
            increment: Increment::One,
            task_balancer: "task_balancer".to_string(),
            target_message: to_binary(&"something").unwrap(),
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // make sure address was saved correctly
        let addr = TARGET_ADO_ADDRESS.load(&deps.storage).unwrap();
        assert_eq!(
            addr,
            AndrAddress {
                identifier: "target_address".to_string(),
            }
        );

        let msg = ExecuteMsg::Execute {};
        let info = mock_info("condition_address", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        println!("{:?}", res.messages);
        let expected = SubMsg::reply_on_error(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "target_address".to_string(),
                msg: to_binary(&counter::ExecuteMsg::IncrementOne {}).unwrap(),
                funds: vec![],
            }),
            1,
        );
        assert_eq!(res.messages, vec![expected])
    }
}
