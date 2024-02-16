use andromeda_std::os::vfs::{validate_path_name, GetLibraryResponse, GetUsernameResponse};
use andromeda_std::{amp::AndrAddr, error::ContractError};
use cosmwasm_std::{Addr, Deps};

use crate::state::{
    get_paths, get_subdir, resolve_pathname, resolve_symlink, PathInfo, ADDRESS_LIBRARY,
    ADDRESS_USERNAME,
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

pub fn get_symlink(deps: Deps, addr: AndrAddr) -> Result<AndrAddr, ContractError> {
    resolve_symlink(deps.storage, deps.api, addr)
}

pub fn get_username(deps: Deps, addr: Addr) -> Result<GetUsernameResponse, ContractError> {
    let username = ADDRESS_USERNAME
        .may_load(deps.storage, addr.to_string().as_str())?
        .unwrap_or(addr.to_string());
    Ok(GetUsernameResponse { username })
}

pub fn get_library_name(deps: Deps, addr: Addr) -> Result<GetLibraryResponse, ContractError> {
    let lib_name = ADDRESS_LIBRARY
        .may_load(deps.storage, addr.to_string().as_str())?
        .unwrap_or(addr.to_string());
    Ok(GetLibraryResponse { library: lib_name })
}
