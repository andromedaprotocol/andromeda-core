use andromeda_std::ado_contract::ADOContract;

use andromeda_std::os::vfs::{
    validate_component_name, validate_path_name, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use andromeda_std::{
    ado_base::InstantiateMsg as BaseInstantiateMsg, common::encode_binary, error::ContractError,
};
use cosmwasm_std::{
    ensure, entry_point, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use crate::state::{add_pathname, resolve_pathname, validate_username, USERS};

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
        ExecuteMsg::RegisterUser { username, address } => {
            execute_register_user(execute_env, username, address)
        }
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
    parent_address: Addr,
) -> Result<Response, ContractError> {
    // validate_component_name(name.clone())?;
    add_pathname(
        execute_env.deps.storage,
        parent_address,
        name,
        execute_env.info.sender,
    )?;
    Ok(Response::default())
}

fn execute_register_user(
    execute_env: ExecuteEnv,
    username: String,
    address: Option<Addr>,
) -> Result<Response, ContractError> {
    let current_user_address = USERS.may_load(execute_env.deps.storage, username.as_str())?;
    if current_user_address.is_some() {
        ensure!(
            current_user_address.unwrap() == execute_env.info.sender,
            ContractError::Unauthorized {}
        );
    }

    validate_username(username.clone())?;
    let address_to_store = address.unwrap_or(execute_env.info.sender);
    USERS.save(
        execute_env.deps.storage,
        username.as_str(),
        &address_to_store,
    )?;

    Ok(Response::default())
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
    }
}

fn query_resolve_path(deps: Deps, path: String) -> Result<Addr, ContractError> {
    validate_path_name(path.clone())?;
    resolve_pathname(deps.storage, deps.api, path)
}
