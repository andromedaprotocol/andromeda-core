use crate::{
    contract::{execute, instantiate},
    state::{resolve_pathname, USERS},
};

use andromeda_std::error::ContractError;
use andromeda_std::os::vfs::{ExecuteMsg, InstantiateMsg};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr,
};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        kernel_address: "kernel".to_string(),
        owner: None,
    };
    let env = mock_env();

    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_register_user() {
    let mut deps = mock_dependencies();
    let username = "user1";
    let sender = "sender";
    let info = mock_info(sender, &[]);
    let env = mock_env();
    let msg = ExecuteMsg::RegisterUser {
        username: username.to_string(),
        address: None,
    };

    execute(deps.as_mut(), env, info, msg).unwrap();

    let saved = USERS.load(deps.as_ref().storage, username).unwrap();
    assert_eq!(saved, sender)
}

#[test]
fn test_register_user_unauthorized() {
    let mut deps = mock_dependencies();
    let username = "user1";
    let sender = "sender";
    let occupier = "occupier";
    let info = mock_info(sender, &[]);
    let env = mock_env();
    let msg = ExecuteMsg::RegisterUser {
        username: username.to_string(),
        address: None,
    };

    USERS
        .save(deps.as_mut().storage, username, &Addr::unchecked(occupier))
        .unwrap();

    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {})
}

#[test]
fn test_register_user_proxy() {
    let mut deps = mock_dependencies();
    let username = "user1";
    let sender = "sender";
    let new_occupier = "occupier";
    let info = mock_info(sender, &[]);
    let env = mock_env();
    let msg = ExecuteMsg::RegisterUser {
        username: username.to_string(),
        address: Some(Addr::unchecked(new_occupier)),
    };

    USERS
        .save(deps.as_mut().storage, username, &Addr::unchecked(sender))
        .unwrap();

    execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    let saved = USERS.load(deps.as_ref().storage, username).unwrap();
    assert_eq!(saved, new_occupier);

    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {})
}

#[test]
fn test_add_path() {
    let mut deps = mock_dependencies();
    let username = "u1";
    let component_name = "f1";
    let sender = "sender";
    let component_addr = Addr::unchecked("f1addr");
    let info = mock_info(sender, &[]);
    let env = mock_env();
    let msg = ExecuteMsg::AddPath {
        name: component_name.to_string(),
        address: component_addr.clone(),
    };

    execute(deps.as_mut(), env, info, msg).unwrap();

    USERS
        .save(deps.as_mut().storage, username, &Addr::unchecked(sender))
        .unwrap();

    let path = format!("/{username}/{component_name}");

    let resolved_addr = resolve_pathname(deps.as_ref().storage, deps.as_ref().api, path).unwrap();

    assert_eq!(resolved_addr, component_addr)
}

#[test]
fn test_add_parent_path() {
    let mut deps = mock_dependencies();
    let username = "u1";
    let user_address = Addr::unchecked("useraddr");
    let component_name = "f1";
    let sender = "sender";
    let info = mock_info(sender, &[]);
    let env = mock_env();
    let msg = ExecuteMsg::AddParentPath {
        name: component_name.to_string(),
        parent_address: user_address.clone(),
    };

    execute(deps.as_mut(), env, info, msg).unwrap();

    USERS
        .save(deps.as_mut().storage, username, &user_address)
        .unwrap();

    let path = format!("/{username}/{component_name}");

    let resolved_addr = resolve_pathname(deps.as_ref().storage, deps.as_ref().api, path).unwrap();

    assert_eq!(resolved_addr, sender)
}
