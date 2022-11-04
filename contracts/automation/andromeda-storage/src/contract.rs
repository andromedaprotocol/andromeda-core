use std::env;

use crate::state::{MAX_PROCESSES, PROCESSES, TASK_BALANCER};
use ado_base::state::ADOContract;
use andromeda_automation::storage::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use common::{ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError};
use cosmwasm_std::{
    ensure, entry_point, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
};
use cw2::{get_contract_version, set_contract_version};

use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-storage";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Store process in a vector
    let process_vec: Vec<Addr> = vec![msg.process];

    MAX_PROCESSES.save(deps.storage, &msg.max_processes)?;
    PROCESSES.save(deps.storage, &process_vec)?;
    TASK_BALANCER.save(deps.storage, &msg.task_balancer)?;

    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "storage".to_string(),
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
        ExecuteMsg::Store { process } => execute_store(deps, env, info, process),
        ExecuteMsg::Remove { process } => execute_remove(deps, env, info, process),
    }
}

fn execute_remove(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    process: String,
) -> Result<Response, ContractError> {
    // Check authority, only task balancer is allowed to remove processes
    let task_balancer = TASK_BALANCER.load(deps.storage)?;
    ensure!(info.sender == task_balancer, ContractError::Unauthorized {});

    // Load existing processes
    let mut processes = PROCESSES.load(deps.storage)?;

    // Find process's index
    let i = processes.iter().position(|x| x == &process);

    // Remove the process if found, else return an error
    if let Some(index) = i {
        processes.swap_remove(index);
        PROCESSES.save(deps.storage, &processes)?;
        Ok(Response::new()
            .add_attribute("action", "removed_process")
            .add_attribute("process", process))
    } else {
        Err(ContractError::ProcessNotFound {})
    }
}

fn execute_store(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    process: String,
) -> Result<Response, ContractError> {
    // Check authority, only task balancer is allowed to store processes
    let task_balancer = TASK_BALANCER.load(deps.storage)?;
    ensure!(info.sender == task_balancer, ContractError::Unauthorized {});

    // Load existing processes
    let mut processes = PROCESSES.load(deps.storage)?;

    // Validate process
    let process = deps.api.addr_validate(&process)?;

    // Check for duplicates
    ensure!(
        !processes.contains(&process),
        ContractError::DuplicateContract {}
    );

    // Add process to processes
    processes.push(process.clone());
    PROCESSES.save(deps.storage, &processes)?;

    Ok(Response::new()
        .add_attribute("action", "stored_process")
        .add_attribute("process", process.to_string()))
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
        QueryMsg::Processes {} => encode_binary(&query_processes(deps)?),
        QueryMsg::TaskBalancer {} => encode_binary(&query_task_balancer(deps)?),
        QueryMsg::FreeSpace {} => encode_binary(&query_free_space(deps)?),
        QueryMsg::HasProcess { process } => encode_binary(&query_has_process(deps, process)?),
    }
}

fn query_has_process(deps: Deps, process: Addr) -> Result<bool, ContractError> {
    // load number of current processes
    let processes = PROCESSES.load(deps.storage)?;
    Ok(processes.contains(&process))
}

fn query_free_space(deps: Deps) -> Result<bool, ContractError> {
    // load number of current processes
    let processes = PROCESSES.load(deps.storage)?;
    let number_of_processes = processes.len();
    let max_processes = MAX_PROCESSES.load(deps.storage)?;
    Ok(max_processes as usize > number_of_processes)
}

fn query_task_balancer(deps: Deps) -> Result<Addr, ContractError> {
    let address = TASK_BALANCER.load(deps.storage)?;
    Ok(address)
}

fn query_processes(deps: Deps) -> Result<Vec<Addr>, ContractError> {
    let addresses = PROCESSES.load(deps.storage)?;
    Ok(addresses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn test_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            task_balancer: Addr::unchecked("task_balancer".to_string()),
            process: Addr::unchecked("process".to_string()),
            max_processes: 3,
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // make sure address was saved correctly
        let processes = PROCESSES.load(&deps.storage).unwrap();
        let expceted_processes = vec![Addr::unchecked("process".to_string())];
        assert_eq!(processes, expceted_processes);

        let task_balancer = TASK_BALANCER.load(&deps.storage).unwrap();
        let expceted_task_balancer = Addr::unchecked("task_balancer".to_string());
        assert_eq!(task_balancer, expceted_task_balancer)
    }

    #[test]
    fn test_store_unauthorized() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            task_balancer: Addr::unchecked("task_balancer".to_string()),
            process: Addr::unchecked("process".to_string()),
            max_processes: 3,
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = ExecuteMsg::Store {
            process: "process".to_string(),
        };
        let info = mock_info("not_task_balancer", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {})
    }

    #[test]
    fn test_store_duplicate() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            task_balancer: Addr::unchecked("task_balancer".to_string()),
            process: Addr::unchecked("process".to_string()),
            max_processes: 3,
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = ExecuteMsg::Store {
            process: "process".to_string(),
        };
        let info = mock_info("task_balancer", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::DuplicateContract {})
    }

    #[test]
    fn test_store_works() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            task_balancer: Addr::unchecked("task_balancer".to_string()),
            process: Addr::unchecked("process".to_string()),
            max_processes: 3,
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = ExecuteMsg::Store {
            process: "process2".to_string(),
        };
        let info = mock_info("task_balancer", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let expected_processes = vec![
            Addr::unchecked("process".to_string()),
            Addr::unchecked("process2".to_string()),
        ];
        let process = PROCESSES.load(&deps.storage).unwrap();
        assert_eq!(process, expected_processes)
    }
}
