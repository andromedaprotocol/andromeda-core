use andromeda_std::os::vfs::validate_path_name;
use andromeda_std::{amp::AndrAddr, error::ContractError};
use cosmwasm_std::{Addr, Deps};

use crate::state::{
    get_paths, get_subdir, resolve_pathname, PathInfo, ADDRESS_LIBRARY, ADDRESS_USERNAME,
};

pub fn resolve_path(deps: Deps, path: AndrAddr) -> Result<Addr, ContractError> {
    validate_path_name(path.to_string())?;
    resolve_pathname(deps.storage, deps.api, path)
}
pub fn subdir(deps: Deps, path: AndrAddr) -> Result<Vec<PathInfo>, ContractError> {
    validate_path_name(path.to_string())?;
    get_subdir(deps.storage, deps.api, path)
}

pub fn paths(deps: Deps, addr: Addr) -> Result<Vec<String>, ContractError> {
    get_paths(deps.storage, addr)
}

pub fn get_username(deps: Deps, addr: Addr) -> Result<String, ContractError> {
    let username = ADDRESS_USERNAME
        .may_load(deps.storage, addr.to_string().as_str())?
        .unwrap_or(addr.to_string());
    Ok(username)
}

pub fn get_library_name(deps: Deps, addr: Addr) -> Result<String, ContractError> {
    let lib_name = ADDRESS_LIBRARY
        .may_load(deps.storage, addr.to_string().as_str())?
        .unwrap_or(addr.to_string());
    Ok(lib_name)
}
