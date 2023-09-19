use andromeda_std::ado_contract::ADOContract;

use andromeda_std::amp::AndrAddr;
use andromeda_std::os::vfs::{
    validate_component_name, validate_path_name, validate_username, ExecuteMsg, InstantiateMsg,
    MigrateMsg, QueryMsg,
};
use andromeda_std::{
    ado_base::InstantiateMsg as BaseInstantiateMsg, common::encode_binary, error::ContractError,
};
use cosmwasm_std::{
    attr, ensure, entry_point, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError,
};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use crate::state::{
    add_pathname, get_paths, get_subdir, paths, resolve_pathname, PathInfo, ADDRESS_USERNAME, USERS,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-vfs";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "vfs".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            kernel_address: msg.kernel_address,
            owner: msg.owner,
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

pub struct ExecuteEnv<'a> {
    deps: DepsMut<'a>,
    pub env: Env,
    pub info: MessageInfo,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let execute_env = ExecuteEnv { deps, env, info };

    match msg {
        ExecuteMsg::AddPath { name, address } => execute_add_path(execute_env, name, address),
        ExecuteMsg::RegisterUser { username } => execute_register_user(execute_env, username),
        ExecuteMsg::AddParentPath {
            name,
            parent_address,
        } => execute_add_parent_path(execute_env, name, parent_address),
    }
}

fn execute_add_path(
    execute_env: ExecuteEnv,
    name: String,
    address: Addr,
) -> Result<Response, ContractError> {
    validate_component_name(name.clone())?;
    add_pathname(
        execute_env.deps.storage,
        execute_env.info.sender,
        name,
        address,
    )?;
    Ok(Response::default())
}

fn execute_add_parent_path(
    execute_env: ExecuteEnv,
    name: String,
    parent_address: AndrAddr,
) -> Result<Response, ContractError> {
    validate_component_name(name.clone())?;
    let parent_address = resolve_pathname(
        execute_env.deps.storage,
        execute_env.deps.api,
        parent_address.into_string(),
    )?;
    let existing = paths()
        .load(
            execute_env.deps.storage,
            &(parent_address.clone(), name.clone()),
        )
        .ok();
    // Ensure that this path is not already added or if already added it should point to same address as above. This prevent external users to override existing paths.
    // Only add path method can override existing paths as its safe because only owner of the path can execute it
    match existing {
        None => {
            add_pathname(
                execute_env.deps.storage,
                parent_address,
                name,
                execute_env.info.sender,
            )?;
        }
        Some(path) => {
            ensure!(
                path.address == execute_env.info.sender,
                ContractError::Unauthorized {}
            )
        }
    };
    Ok(Response::default())
}

fn execute_register_user(
    execute_env: ExecuteEnv,
    username: String,
) -> Result<Response, ContractError> {
    let current_user_address = USERS.may_load(execute_env.deps.storage, username.as_str())?;
    if current_user_address.is_some() {
        ensure!(
            current_user_address.unwrap() == execute_env.info.sender,
            ContractError::Unauthorized {}
        );
    }

    //Remove username registration from previous username
    USERS.remove(execute_env.deps.storage, username.as_str());

    validate_username(username.clone())?;
    USERS.save(
        execute_env.deps.storage,
        username.as_str(),
        &execute_env.info.sender,
    )?;
    //Update current address' username
    ADDRESS_USERNAME.save(
        execute_env.deps.storage,
        execute_env.info.sender.as_ref(),
        &username,
    )?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "register_username"),
        attr("addr", execute_env.info.sender),
        attr("username", username),
    ]))
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::ResolvePath { path } => encode_binary(&query_resolve_path(deps, path)?),
        QueryMsg::SubDir { path } => encode_binary(&query_subdir(deps, path)?),
        QueryMsg::Paths { addr } => encode_binary(&query_paths(deps, addr)?),
        QueryMsg::GetUsername { address } => encode_binary(&query_get_username(deps, address)?),
    }
}

fn query_resolve_path(deps: Deps, path: String) -> Result<Addr, ContractError> {
    validate_path_name(path.clone())?;
    resolve_pathname(deps.storage, deps.api, path)
}
fn query_subdir(deps: Deps, path: String) -> Result<Vec<PathInfo>, ContractError> {
    validate_path_name(path.clone())?;
    get_subdir(deps.storage, deps.api, path)
}

fn query_paths(deps: Deps, addr: Addr) -> Result<Vec<String>, ContractError> {
    get_paths(deps.storage, addr)
}

fn query_get_username(deps: Deps, addr: Addr) -> Result<String, ContractError> {
    let username = ADDRESS_USERNAME
        .may_load(deps.storage, addr.to_string().as_str())?
        .unwrap_or(addr.to_string());
    Ok(username)
}
