use crate::state::{QUERY_MSG, TARGET_ADO_ADDRESS};
use ado_base::state::ADOContract;
use andromeda_automation::oracle::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use common::{ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError};
use serde::Deserialize;
use std::env;

use cosmwasm_std::{
    ensure, entry_point, Binary, Deps, DepsMut, Env, MessageInfo, QueryRequest, Reply, Response,
    StdError, WasmQuery,
};
use cw2::{get_contract_version, set_contract_version};

use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-oracle";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    QUERY_MSG.save(deps.storage, &msg.message_binary)?;
    TARGET_ADO_ADDRESS.save(deps.storage, &msg.target_address)?;

    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "oracle".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: None,
            primitive_contract: None,
        },
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, _msg: Reply) -> Result<Response, ContractError> {
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
        QueryMsg::CurrentTarget {} => encode_binary(&query_current_target(deps)?),
        QueryMsg::Target {} => encode_binary(&query_target(deps)?),
    }
}

fn query_current_target(deps: Deps) -> Result<String, ContractError> {
    let address = TARGET_ADO_ADDRESS.load(deps.storage)?;
    Ok(address.identifier)
}

fn query_target<T>(deps: Deps) -> Result<T, ContractError>
where
    T: for<'a> Deserialize<'a>,
{
    let contract_addr = TARGET_ADO_ADDRESS.load(deps.storage)?.identifier;
    let msg = QUERY_MSG.load(deps.storage)?;

    let response: T = deps
        .querier
        .query(&QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }))?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::app::AndrAddress;
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        to_binary,
    };

    #[test]
    fn test_initialization() {
        let mut deps = mock_dependencies();
        let target_address = AndrAddress {
            identifier: "target_address".to_string(),
        };
        let message_binary = to_binary(&"binary").unwrap();

        let msg = InstantiateMsg {
            target_address,
            message_binary: message_binary.clone(),
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
        let message = QUERY_MSG.load(&deps.storage).unwrap();
        assert_eq!(message, message_binary)
    }
}
