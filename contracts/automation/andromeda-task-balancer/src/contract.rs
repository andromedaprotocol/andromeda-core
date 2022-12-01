use crate::state::{State, STATE, STORAGE_CONTRACTS, UP_NEXT};
use ado_base::state::ADOContract;
use andromeda_app::app::QueryMsg::GetAddresses;
use andromeda_automation::storage::ExecuteMsg as StorageExecuteMsg;
use andromeda_automation::storage::InstantiateMsg as StorageInstantiateMsg;
use andromeda_automation::storage::QueryMsg as StorageQueryMsg;
use andromeda_automation::task_balancer::{
    ExecuteMsg, GetSizeResponse, GetStorageResponse, InstantiateMsg, MigrateMsg, QueryMsg,
};
use common::response::get_reply_address;
use common::{ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError};
use cosmwasm_std::Addr;
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
const INSTANTIATED_CONTRACT_REPLY_ID: u64 = 1;

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
    let contract_address = get_reply_address(msg.clone())?;

    let state = STATE.load(deps.storage)?;
    let contracts = state.contracts;
    let storage_contracts = STORAGE_CONTRACTS.may_load(deps.storage)?;

    match msg.id {
        INSTANTIATED_CONTRACT_REPLY_ID => {
            if let Some(mut storage_contracts) = storage_contracts {
                storage_contracts.push(contract_address.to_string());
                STORAGE_CONTRACTS.save(deps.storage, &storage_contracts)?;
            } else {
                STORAGE_CONTRACTS.save(deps.storage, &vec![contract_address.to_string()])?;
            }
            Ok(Response::new()
                .add_attribute("action", "stored_storage_contract_address")
                .add_attribute("storage_address", contract_address)
                .add_attribute("number_of_contracts", contracts))
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
        ExecuteMsg::Add { process } => add_process(deps, env, info, process),
        ExecuteMsg::Remove { process } => remove_process(deps, env, info, process),
    }
}

fn remove_process(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    process: String,
) -> Result<Response, ContractError> {
    // Add permission for removal of processes
    // Sender should be part of an already existing process, and can't request the removal of another process
    let app_addresses: Vec<Addr> = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: process.clone(),
        msg: to_binary(&GetAddresses {})?,
    }))?;
    ensure!(
        app_addresses.contains(&info.sender),
        ContractError::Unauthorized {}
    );

    // Identify which storage contract a certain process belongs to
    // Get storage address that holds the process
    let mut num = 0;
    let contract_addr = STORAGE_CONTRACTS.load(deps.storage)?;

    let storage_address = loop {
        let has_process = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: contract_addr[num as usize].clone(),
            msg: to_binary(&StorageQueryMsg::HasProcess {
                process: deps.api.addr_validate(&process)?,
            })?,
        }))?;
        if has_process {
            break &contract_addr[num as usize];
        }
        num += 1;

        if num as usize >= (contract_addr.len()) {
            return Err(ContractError::ProcessNotFound {});
        }
    };
    // Mark that storage contract by adding it to the UP_NEXT vector, which allows us to fill that empty space in the future
    let up_next = UP_NEXT.may_load(deps.storage)?;

    if let Some(mut up_next) = up_next {
        up_next.push(storage_address.clone());
        UP_NEXT.save(deps.storage, &up_next)?;
    } else {
        let up_next = vec![storage_address.clone()];
        UP_NEXT.save(deps.storage, &up_next)?;
    }

    Ok(Response::new()
        .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: storage_address.clone(),
            msg: to_binary(&StorageExecuteMsg::Remove {
                process: process.to_string(),
            })?,
            funds: vec![],
        })))
        .add_attribute("action", "removed_process")
        .add_attribute("process", process)
        .add_attribute("up_next", storage_address))
}

fn add_process(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    process: String,
) -> Result<Response, ContractError> {
    // Not anyone should be allowed to add tasks to tree
    let contract = ADOContract::default();
    ensure!(
        contract.is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    // Validate the process's address
    let process = deps.api.addr_validate(&process)?;

    // In case no storage contracts have been instantiated yet
    let state = STATE.load(deps.storage)?;
    if state.contracts == Uint128::zero() {
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
                max_processes: state.max,
            })?,
            funds: vec![],
            label: "storage".to_string(),
        });
        return Ok(Response::new()
            .add_attribute("action", "try_add")
            .add_submessage(SubMsg::reply_on_success(
                msg,
                INSTANTIATED_CONTRACT_REPLY_ID,
            )));
    }

    // Check if there are any earlier storage contracts with free space
    let up_next = UP_NEXT.may_load(deps.storage)?;

    if let Some(mut up_next) = up_next {
        if !up_next.is_empty() {
            // We access index 0 since it was the earliest storage contract to join the list
            let contract_addr = &up_next[0];

            // Execute addition of task contract to a storage contract
            let msg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&StorageExecuteMsg::Store {
                    process: process.to_string(),
                })?,
                funds: vec![],
            });

            // Remove storage contract from up next
            up_next.remove(0);
            UP_NEXT.save(deps.storage, &up_next)?;

            return Ok(Response::new()
                .add_attribute("action", "try_add")
                .add_message(msg));
        }
    };

    let storage_contracts = STORAGE_CONTRACTS.load(deps.storage)?;
    let number_of_contracts = storage_contracts.len();
    // Get latest contract
    let storage_address = &storage_contracts[number_of_contracts - 1_usize];

    // queries specified contract to find if full
    // Checks if contract is full or not
    let free_space: bool = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: storage_address.clone(),
        msg: to_binary(&StorageQueryMsg::FreeSpace {})?,
    }))?;

    // In case there's no free space in the latest contract, instantiate a new one
    if !free_space {
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
                max_processes: state.max,
            })?,
            funds: vec![],
            label: "storage".to_string(),
        });
        Ok(Response::new()
            .add_attribute("action", "try_add")
            .add_submessage(SubMsg::reply_on_success(
                msg,
                INSTANTIATED_CONTRACT_REPLY_ID,
            )))
    } else {
        // Execute addition of task contract to a storage contract
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: storage_address.to_string(),
            msg: to_binary(&StorageExecuteMsg::Store {
                process: process.to_string(),
            })?,
            funds: vec![],
        });
        Ok(Response::new()
            .add_attribute("action", "try_add")
            .add_message(msg))
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
        QueryMsg::GetSize {} => encode_binary(&query_count(deps)?),
        QueryMsg::Storage {} => encode_binary(&query_storage_contracts(deps)?),
        QueryMsg::UpNext {} => encode_binary(&query_up_next(deps)?),
    }
}

fn query_up_next(deps: Deps) -> Result<Vec<String>, ContractError> {
    let up_next = UP_NEXT.load(deps.storage)?;
    Ok(up_next)
}

fn query_storage_contracts(deps: Deps) -> Result<GetStorageResponse, ContractError> {
    let storage_addresses = STORAGE_CONTRACTS.load(deps.storage)?;
    Ok(GetStorageResponse { storage_addresses })
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
    use crate::contract::instantiate;
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
        };
        assert_eq!(state, expected_state)
    }
}
