use andromeda_std::ado_contract::ADOContract;
use andromeda_std::amp::AndrAddr;
use andromeda_std::error::ContractError;
use andromeda_std::os::vfs::{validate_component_name, validate_username};
use cosmwasm_std::{attr, ensure, Addr, DepsMut, Env, MessageInfo, Response};

use crate::state::{
    add_pathname, paths, resolve_pathname, ADDRESS_LIBRARY, ADDRESS_USERNAME, LIBRARIES, USERS,
};

pub struct ExecuteEnv<'a> {
    pub deps: DepsMut<'a>,
    pub env: Env,
    pub info: MessageInfo,
}

pub fn add_path(
    env: ExecuteEnv,
    name: String,
    address: Addr,
    parent_address: Option<Addr>,
) -> Result<Response, ContractError> {
    let kernel_address = ADOContract::default().get_kernel_address(env.deps.storage)?;
    ensure!(
        parent_address.is_none()
            || env.info.sender == kernel_address
            || ADOContract::default()
                .is_contract_owner(env.deps.storage, env.info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    validate_component_name(name.clone())?;
    add_pathname(
        env.deps.storage,
        parent_address.unwrap_or(env.info.sender),
        name,
        address,
    )?;
    Ok(Response::default())
}

pub fn add_parent_path(
    env: ExecuteEnv,
    name: String,
    parent_address: AndrAddr,
) -> Result<Response, ContractError> {
    validate_component_name(name.clone())?;
    let parent_address = resolve_pathname(env.deps.storage, env.deps.api, parent_address)?;
    let existing = paths()
        .load(env.deps.storage, &(parent_address.clone(), name.clone()))
        .ok();
    // Ensure that this path is not already added or if already added it should point to same address as above. This prevent external users to override existing paths.
    // Only add path method can override existing paths as its safe because only owner of the path can execute it
    match existing {
        None => {
            add_pathname(env.deps.storage, parent_address, name, env.info.sender)?;
        }
        Some(path) => {
            ensure!(
                path.address == env.info.sender,
                ContractError::Unauthorized {}
            )
        }
    };
    Ok(Response::default())
}

pub fn register_user(env: ExecuteEnv, username: String) -> Result<Response, ContractError> {
    let current_user_address = USERS.may_load(env.deps.storage, username.as_str())?;
    if current_user_address.is_some() {
        ensure!(
            current_user_address.unwrap() == env.info.sender,
            ContractError::Unauthorized {}
        );
    }

    //Remove username registration from previous username
    USERS.remove(env.deps.storage, username.as_str());

    validate_username(username.clone())?;
    USERS.save(env.deps.storage, username.as_str(), &env.info.sender)?;
    //Update current address' username
    ADDRESS_USERNAME.save(env.deps.storage, env.info.sender.as_ref(), &username)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "register_username"),
        attr("addr", env.info.sender),
        attr("username", username),
    ]))
}

pub fn register_library(
    env: ExecuteEnv,
    lib_name: String,
    lib_address: Addr,
) -> Result<Response, ContractError> {
    let kernel_address = ADOContract::default().get_kernel_address(env.deps.storage)?;
    ensure!(
        env.info.sender == kernel_address
            || ADOContract::default()
                .is_contract_owner(env.deps.storage, env.info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    validate_username(lib_name.clone())?;
    LIBRARIES.save(env.deps.storage, lib_name.as_str(), &lib_address)?;
    //Update current address' username
    ADDRESS_LIBRARY.save(env.deps.storage, lib_address.as_str(), &lib_name)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "register_library"),
        attr("addr", lib_address),
        attr("library_name", lib_name),
    ]))
}
