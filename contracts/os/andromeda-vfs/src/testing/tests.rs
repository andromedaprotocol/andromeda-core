use crate::{
    contract::{execute, instantiate, query},
    state::{add_pathname, resolve_pathname, PathInfo, ADDRESS_USERNAME, USERS},
};

use andromeda_std::os::vfs::{ExecuteMsg, InstantiateMsg};
use andromeda_std::{error::ContractError, os::vfs::QueryMsg};
use cosmwasm_std::{
    from_binary,
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
    };

    USERS
        .save(deps.as_mut().storage, username, &Addr::unchecked(occupier))
        .unwrap();

    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {})
}

#[test]
fn test_register_user_already_registered() {
    let mut deps = mock_dependencies();
    let username = "user1";
    let new_username = "user2";
    let sender = "sender";
    let info = mock_info(sender, &[]);
    let env = mock_env();
    let msg = ExecuteMsg::RegisterUser {
        username: new_username.to_string(),
    };

    USERS
        .save(deps.as_mut().storage, username, &Addr::unchecked(sender))
        .unwrap();

    execute(deps.as_mut(), env, info, msg).unwrap();
    let addr = USERS.load(deps.as_ref().storage, new_username).unwrap();
    assert_eq!(addr, sender);
    let username = ADDRESS_USERNAME
        .load(deps.as_ref().storage, sender)
        .unwrap();
    assert_eq!(username, new_username)
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

#[test]
fn test_get_username() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let username = "u1";
    let sender = "sender";

    ADDRESS_USERNAME
        .save(deps.as_mut().storage, sender, &username.to_string())
        .unwrap();

    let query_msg = QueryMsg::GetUsername {
        address: Addr::unchecked(sender),
    };

    let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let val: String = from_binary(&res).unwrap();

    assert_eq!(val, username);

    let unregistered_addr = "notregistered";
    let query_msg = QueryMsg::GetUsername {
        address: Addr::unchecked(unregistered_addr),
    };

    let res = query(deps.as_ref(), env, query_msg).unwrap();
    let val: String = from_binary(&res).unwrap();

    assert_eq!(val, unregistered_addr);
}

#[test]
fn test_get_subdir() {
    let mut deps = mock_dependencies();
    let username = "u1";
    let sender = Addr::unchecked("sender");
    let env = mock_env();
    let root_paths = vec![
        PathInfo {
            name: "f1".to_string(),
            address: Addr::unchecked("f1addr"),
            parent_address: sender.clone(),
        },
        PathInfo {
            name: "f2".to_string(),
            address: Addr::unchecked("f2addr"),
            parent_address: sender.clone(),
        },
    ];
    let sub_paths = vec![
        PathInfo {
            name: "sub1".to_string(),
            address: Addr::unchecked("sub1addr"),
            parent_address: root_paths[0].address.clone(),
        },
        PathInfo {
            name: "sub2".to_string(),
            address: Addr::unchecked("sub2addr"),
            parent_address: root_paths[0].address.clone(),
        },
    ];

    USERS
        .save(deps.as_mut().storage, username, &sender)
        .unwrap();

    // Add all root components
    for path in root_paths.clone() {
        let _ = add_pathname(
            deps.as_mut().storage,
            sender.clone(),
            path.name,
            path.address,
        );
    }

    for path in sub_paths.clone() {
        let _ = add_pathname(
            deps.as_mut().storage,
            path.parent_address.clone(),
            path.name,
            path.address,
        );
    }

    for path in root_paths.clone() {
        let path_name = format!("/{username}/{name}", name = path.name);
        let resolved_addr =
            resolve_pathname(deps.as_ref().storage, deps.as_ref().api, path_name.clone());
        assert!(resolved_addr.is_ok(), "{path_name} not found");
        assert_eq!(resolved_addr.unwrap(), path.address)
    }

    let query_msg = QueryMsg::SubDir {
        path: format!("/{username}"),
    };
    let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let val: Vec<PathInfo> = from_binary(&res).unwrap();
    assert_eq!(val, root_paths.clone());

    let subdir = &root_paths[0].name;
    let query_msg = QueryMsg::SubDir {
        path: format!("/{username}/{subdir}"),
    };
    let res = query(deps.as_ref(), env, query_msg).unwrap();
    let val: Vec<PathInfo> = from_binary(&res).unwrap();
    assert_eq!(val, sub_paths.clone());
}

#[test]
fn test_get_paths() {
    let mut deps = mock_dependencies();
    let username = "u1";
    let sender = Addr::unchecked("sender");
    let env = mock_env();
    let root_paths = vec![
        PathInfo {
            name: "f1".to_string(),
            address: Addr::unchecked("f1addr"),
            parent_address: sender.clone(),
        },
        PathInfo {
            name: "f2".to_string(),
            address: Addr::unchecked("f2addr"),
            parent_address: sender.clone(),
        },
    ];
    let sub_paths = vec![
        PathInfo {
            name: "sub1".to_string(),
            address: Addr::unchecked("sub1addr"),
            parent_address: root_paths[0].address.clone(),
        },
        PathInfo {
            name: "sub2".to_string(),
            address: Addr::unchecked("sub2addr"),
            parent_address: root_paths[0].address.clone(),
        },
    ];

    USERS
        .save(deps.as_mut().storage, username, &sender)
        .unwrap();
    ADDRESS_USERNAME
        .save(deps.as_mut().storage, &sender.as_str(), &username.to_string())
        .unwrap();

    // Add all root components
    for path in root_paths.clone() {
        let _ = add_pathname(
            deps.as_mut().storage,
            sender.clone(),
            path.name,
            path.address.clone(),
        );
        for sub_path in sub_paths.clone() {
            let _ = add_pathname(
                deps.as_mut().storage,
                path.address.clone(),
                sub_path.name,
                sub_path.address,
            );
        }
    }

    for path in root_paths.clone() {
        let path_name = format!("/{username}/{name}", name = path.name);
        let resolved_addr =
            resolve_pathname(deps.as_ref().storage, deps.as_ref().api, path_name.clone());
        assert!(resolved_addr.is_ok(), "{path_name} not found");
        assert_eq!(resolved_addr.unwrap(), path.address)
    }

    let query_msg = QueryMsg::Paths {
        addr: sub_paths[0].address.clone(),
    };
    let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let val: Vec<String> = from_binary(&res).unwrap();
    assert_eq!(val.len(),2);
}
