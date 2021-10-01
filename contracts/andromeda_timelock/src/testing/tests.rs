use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{ Response, coin, };
use cw721::Expiration;
use crate::contract::{ instantiate, execute };
use crate::msg::{ ExecuteMsg, InstantiateMsg };
use crate::state::{ STATE, };

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let owner = "owner";
    let info = mock_info(owner, &[]);
    let msg = InstantiateMsg {};
    let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());
    //checking
    let state = STATE.load(deps.as_ref().storage).unwrap();
    assert!( state.owner == owner );
}

#[test]
fn test_execute_hold_funds() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let owner = "owner";
    let info = mock_info(owner, &vec![coin(1000u128, "uusd")]);
    let msg = ExecuteMsg::HoldFunds {
        expire: Expiration::Never{}
    };

    //add address for registered moderator

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!( Response::default(), res );
}

#[test]
fn test_execute_release_funds() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let owner = "owner";

    let info = mock_info(owner, &vec![coin(1000u128, "uusd")]);
    let msg = ExecuteMsg::HoldFunds {
        expire: Expiration::Never{}
    };

    //add address for registered moderator
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    let info = mock_info(owner, &[]);
    let msg = ExecuteMsg::ReleaseFunds {};
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_ne!( Response::default(), res );
}