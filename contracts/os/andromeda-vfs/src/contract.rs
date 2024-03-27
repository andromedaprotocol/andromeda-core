use andromeda_std::ado_contract::ADOContract;

use andromeda_std::common::migrate::{migrate as do_migrate, MigrateMsg};
use andromeda_std::os::vfs::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{
    ado_base::InstantiateMsg as BaseInstantiateMsg, common::encode_binary, error::ContractError,
};

use cosmwasm_std::{
    entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
};
use cw2::set_contract_version;

use crate::{execute, query};

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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let execute_env = execute::ExecuteEnv { deps, env, info };

    match msg {
        ExecuteMsg::AddPath {
            name,
            address,
            parent_address,
        } => execute::add_path(execute_env, name, address, parent_address),
        ExecuteMsg::AddSymlink {
            name,
            symlink,
            parent_address,
        } => execute::add_symlink(execute_env, name, symlink, parent_address),
        ExecuteMsg::RegisterUser { username, address } => {
            execute::register_user(execute_env, username, address)
        }
        ExecuteMsg::AddParentPath {
            name,
            parent_address,
        } => execute::add_parent_path(execute_env, name, parent_address),
        ExecuteMsg::RegisterLibrary {
            lib_name,
            lib_address,
        } => execute::register_library(execute_env, lib_name, lib_address),
        ExecuteMsg::RegisterUserCrossChain { chain, address } => {
            execute::register_user_cross_chain(execute_env, chain, address)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    do_migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::ResolvePath { path } => encode_binary(&query::resolve_path(deps, path)?),
        QueryMsg::SubDir { path } => encode_binary(&query::subdir(deps, path)?),
        QueryMsg::Paths { addr } => encode_binary(&query::paths(deps, addr)?),
        QueryMsg::GetUsername { address } => encode_binary(&query::get_username(deps, address)?),
        QueryMsg::GetLibrary { address } => encode_binary(&query::get_library_name(deps, address)?),
        QueryMsg::ResolveSymlink { path } => encode_binary(&query::get_symlink(deps, path)?),
    }
}
