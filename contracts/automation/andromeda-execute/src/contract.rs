use std::env;

use crate::state::{CONDITION_ADO_ADDRESS, TARGET_ADO_ADDRESS};
use ado_base::state::ADOContract;
use andromeda_automation::{
    counter,
    execute::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
};
use common::{
    ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError, require,
};
use cosmwasm_std::{
    ensure, entry_point, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, SubMsg, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
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
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    Ok(Response::default())
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
        ExecuteMsg::Execute {} => execute_execute(deps, env, info),
    }
}

fn execute_execute(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let app_contract = contract.get_app_contract(deps.storage)?;

    let condition_ado = CONDITION_ADO_ADDRESS.load(deps.storage)?.get_address(
        deps.api,
        &deps.querier,
        app_contract.clone(),
    )?;

    ensure!(info.sender == condition_ado, ContractError::Unauthorized {});

    let contract_addr = TARGET_ADO_ADDRESS.load(deps.storage)?.get_address(
        deps.api,
        &deps.querier,
        app_contract,
    )?;

    Ok(
        Response::new().add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg: to_binary(&counter::ExecuteMsg::Increment {})?,
            funds: vec![],
        }))),
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // New version
    let version: Version = CONTRACT_VERSION.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    let contract = ADOContract::default();

    require(
        stored.contract == CONTRACT_NAME,
        ContractError::CannotMigrate {
            previous_contract: stored.contract,
        },
    )?;

    // New version has to be newer/greater than the old version
    require(
        storage_version < version,
        ContractError::CannotMigrate {
            previous_contract: stored.version,
        },
    )?;

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
        QueryMsg::TargetADO {} => encode_binary(&query_execute_ado_query(deps)?),
    }
}

fn query_execute_ado_query(deps: Deps) -> Result<String, ContractError> {
    let address = TARGET_ADO_ADDRESS.load(deps.storage)?;
    Ok(address.identifier)
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let expected = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "target_address".to_string(),
            msg: to_binary(&counter::ExecuteMsg::Increment {}).unwrap(),
            funds: vec![],
        }));
        assert_eq!(res.messages, vec![expected])
    }
}
