
use andromeda_protocol::modules::whitelist::{ Whitelist, WHITELIST };
use cosmwasm_std::Response;
use crate::{
    contract::{instantiate, execute},
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg},
    state::{State, STATE},
};

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        creator: String::from("creator"),
        moderators: vec!["11".to_string(), "22".to_string()],
    };
    let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_whitelist() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();

    let moderator = "creator";
    let info = mock_info(moderator.clone(), &[]);

    let whitelistee = "whitelistee";

    //input moderator for test

    let state = State {
        creator: moderator.to_string(),
        whitelist: Whitelist {
            moderators: vec![moderator.to_string()],
        },
    };

    STATE.save(deps.as_mut().storage, &state).unwrap();

    let msg = ExecuteMsg::Whitelist {
        address: whitelistee.to_string(),
    };

    //add address for registered moderator

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(Response::default(), res);

    let whitelisted = WHITELIST
        .load(deps.as_ref().storage, whitelistee.to_string())
        .unwrap();
    assert_eq!(true, whitelisted);

    let whitelisted = WHITELIST
        .load(deps.as_ref().storage, "111".to_string())
        .unwrap_err();

    match whitelisted {
        cosmwasm_std::StdError::NotFound { .. } => {
            assert_eq!(false, false);
        }
        _ => {
            assert_eq!(false, true);
        }
    }

    // assert_eq!(cosmwasm_std::StdError::NotFound {kind: }, whitelisted);

    //add address for unregistered moderator
    let unauth_info = mock_info("anyone", &[]);
    let res =
        execute(deps.as_mut(), env.clone(), unauth_info.clone(), msg.clone()).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, res);
}

#[test]
fn test_remove_whitelist() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();

    let moderator = "creator";
    let info = mock_info(moderator.clone(), &[]);

    let whitelistee = "whitelistee";

    //save moderator

    let state = State {
        creator: moderator.to_string(),
        whitelist: Whitelist {
            moderators: vec![moderator.to_string()],
        },
    };

    STATE.save(deps.as_mut().storage, &state).unwrap();

    let msg = ExecuteMsg::RemoveWhitelist {
        address: whitelistee.to_string(),
    };

    //add address for registered moderator
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(Response::default(), res);

    let whitelisted = WHITELIST
        .load(deps.as_ref().storage, whitelistee.to_string())
        .unwrap();
    assert_eq!(false, whitelisted);

    //add address for unregistered moderator
    let unauth_info = mock_info("anyone", &[]);
    let res =
        execute(deps.as_mut(), env.clone(), unauth_info.clone(), msg.clone()).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, res);
}