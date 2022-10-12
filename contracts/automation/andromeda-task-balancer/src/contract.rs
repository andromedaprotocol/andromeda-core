use crate::state::{State, CONTRACTS, STATE, STORAGE_CONTRACT};
use ado_base::state::ADOContract;
use andromeda_automation::storage::ExecuteMsg as StorageExecuteMsg;
use andromeda_automation::storage::InstantiateMsg as StorageInstantiateMsg;

use andromeda_automation::task_balancer::{
    ExecuteMsg, GetSizeResponse, GetStorageResponse, InstantiateMsg, LoopQueryMsg, MigrateMsg,
    QueryMsg,
};
use common::{ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError};
use cosmwasm_std::{
    ensure, entry_point, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    QueryRequest, Reply, Response, StdError, SubMsg, Uint128, WasmMsg, WasmQuery,
};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;
use std::env;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-task-balancer";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        contracts: Uint128::zero(),
        max: msg.max,
        storage_code_id: msg.storage_code_id,
        admin: info.sender.to_string(),
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;
    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "task-balancer".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: None,
            primitive_contract: None,
        },
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let response = msg.result;
    // We only get a reply on success, so it's safe to assume there's an event
    // There's also only one event resulting from instantiation, so we access the first (and only) event
    let address = &response.unwrap().events[0];
    // According to the raw logs, the first key value pair holds the instantiated contract's address
    let attribute = &address.attributes[0];
    let contract_address = &attribute.value;

    match msg.id {
        1 => {
            STORAGE_CONTRACT.save(deps.storage, contract_address)?;
            Ok(Response::new()
                .add_attribute("action", "stored_storage_contract_address")
                .add_attribute("storage_address", contract_address))
        }
        _ => Ok(Response::default()),
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
        ExecuteMsg::Add { contract } => try_add(deps, env, info, contract),
        ExecuteMsg::UpdateAdmin { new_admin } => try_update(deps, info, new_admin),
        ExecuteMsg::RemoveProcess { process_address } => {
            remove_process(deps, env, info, process_address)
        }
    }
}

fn remove_process(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    process_address: String,
) -> Result<Response, ContractError> {
    // Load storage contract's address
    let contract_addr = STORAGE_CONTRACT.load(deps.storage)?;

    Ok(Response::new()
        .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg: to_binary(&StorageExecuteMsg::Remove {
                process: process_address.clone(),
            })?,
            funds: vec![],
        })))
        .add_attribute("action", "removed_process")
        .add_attribute("process", process_address))
}

fn try_add(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    process: String,
) -> Result<Response, ContractError> {
    // Not anyone should be allowed to add tasks to tree
    let state = STATE.load(deps.storage)?;
    ensure!(info.sender == state.admin, ContractError::Unauthorized {});

    // Validate the process's address
    let process = deps.api.addr_validate(&process)?;

    // Task balancing variable creation
    let mut num = Uint128::from(env.block.height) % state.contracts;
    let mut count = Uint128::zero();

    loop {
        // Get contract address from MAP
        let address = CONTRACTS.load(deps.storage, num.to_string())?;

        // queries specified contract to find if full
        // Checks if contract is full or not
        let total: GetSizeResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: address.clone(),
            msg: to_binary(&LoopQueryMsg::GetSize {})?,
        }))?;
        if count > state.contracts {
            // Instantiate new contract and add to storage
            // break from loop
            // Instantiation of contract should  directly store with it the task contract in question
            STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
                state.contracts += Uint128::from(1u64);
                Ok(state)
            })?;
            let state = STATE.load(deps.storage)?;
            let msg = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: None,
                code_id: state.storage_code_id,
                msg: to_binary(&StorageInstantiateMsg {
                    task_balancer: env.contract.address,
                    process,
                })?,
                funds: vec![],
                label: "storage".to_string(),
            });
            return Ok(Response::new()
                .add_attribute("action", "try_add")
                .add_submessage(SubMsg::reply_on_success(msg, 1)));
        }
        if total.size >= Uint128::from(state.max) {
            num += Uint128::from(1u64);
            count += Uint128::from(1u64);
            // Repeat num for total number of contracts
            num %= state.contracts;
            continue;
        }
        if total.size < Uint128::from(state.max) {
            // Execute addition of task contract to a storage contract
            let msg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: address,
                msg: to_binary(&StorageExecuteMsg::Store {
                    process: process.to_string(),
                })?,
                funds: vec![],
            });
            return Ok(Response::new()
                .add_attribute("action", "try_add")
                .add_message(msg));
        }
    }
}

pub fn try_update(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        ensure!(info.sender == state.admin, ContractError::Unauthorized {});

        state.admin = new_admin;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("action", "new_admin"))
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
        QueryMsg::GetSize {} => encode_binary(&query_count(deps)?),
        QueryMsg::Storage {} => encode_binary(&query_storage(deps)?),
    }
}

fn query_storage(deps: Deps) -> Result<GetStorageResponse, ContractError> {
    let storage_address = STORAGE_CONTRACT.load(deps.storage)?;
    Ok(GetStorageResponse { storage_address })
}

fn query_count(deps: Deps) -> Result<GetSizeResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    Ok(GetSizeResponse {
        size: state.contracts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn test_initialization() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            max: 5,
            storage_code_id: 1,
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // make sure address was saved correctly
        let state = STATE.load(&deps.storage).unwrap();
        let expected_state = State {
            contracts: Uint128::zero(),
            max: 5,
            storage_code_id: 1,
            admin: "creator".to_string(),
        };
        assert_eq!(state, expected_state)
    }

    #[test]
    fn test_update_unauthorized() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            max: 5,
            storage_code_id: 1,
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let msg = ExecuteMsg::UpdateAdmin {
            new_admin: "new_admin".to_string(),
        };
        let info = mock_info("random", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {})
    }

    #[test]
    fn test_update_works() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            max: 5,
            storage_code_id: 1,
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let msg = ExecuteMsg::UpdateAdmin {
            new_admin: "new_admin".to_string(),
        };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let state = STATE.load(&deps.storage).unwrap();
        let expected_state = State {
            contracts: Uint128::zero(),
            max: 5,
            storage_code_id: 1,
            admin: "new_admin".to_string(),
        };
        assert_eq!(state, expected_state)
    }
}
