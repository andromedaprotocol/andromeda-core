#[cfg(test)]
use andromeda_std::testing::mock_querier::{mock_dependencies_custom, MOCK_KERNEL_CONTRACT};
use cosmwasm_std::{coin, Addr, Uint128};

use crate::contract::{execute, instantiate};
use crate::state::BALANCES;



use andromeda_std::os::economics::{ExecuteMsg, InstantiateMsg};

use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
    };
    let env = mock_env();

    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_deposit() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    //Test No Coin Deposit
    let info = mock_info("creator", &[]);
    let msg = ExecuteMsg::Deposit { address: None };
    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert!(res.is_err());

    // Test Single Coin Deposit
    let info = mock_info("creator", &[coin(100, "uandr")]);
    let msg = ExecuteMsg::Deposit { address: None };
    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let balance = BALANCES
        .load(
            deps.as_ref().storage,
            (Addr::unchecked("creator"), "uandr".to_string()),
        )
        .unwrap();

    assert_eq!(balance, Uint128::from(100u128));

    // Test Multiple Coin Deposit
    let info = mock_info("creator", &[coin(100, "uandr"), coin(100, "uusd")]);
    let msg = ExecuteMsg::Deposit { address: None };
    execute(deps.as_mut(), env, info, msg).unwrap();

    let balance = BALANCES
        .load(
            deps.as_ref().storage,
            (Addr::unchecked("creator"), "uandr".to_string()),
        )
        .unwrap();
    assert_eq!(balance, Uint128::from(200u128));

    let balance = BALANCES
        .load(
            deps.as_ref().storage,
            (Addr::unchecked("creator"), "uusd".to_string()),
        )
        .unwrap();
    assert_eq!(balance, Uint128::from(100u128));
}

#[test]
fn test_pay_fee() {}
