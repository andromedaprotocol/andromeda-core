use andromeda_std::amp::AndrAddr;
use andromeda_std::error::ContractError;
#[cfg(test)]
use andromeda_std::testing::mock_querier::{
    mock_dependencies_custom, MOCK_ACTION, MOCK_KERNEL_CONTRACT,
};
use andromeda_std::testing::mock_querier::{MOCK_ADO_PUBLISHER, MOCK_APP_CONTRACT};
use cosmwasm_std::{coin, to_json_binary, Addr, BankMsg, CosmosMsg, Uint128};
use cw20::Cw20ReceiveMsg;

use crate::contract::{cw20_withdraw_msg, execute, instantiate, spend_balance};
use crate::state::BALANCES;

use andromeda_std::os::economics::{Cw20HookMsg, ExecuteMsg, InstantiateMsg};

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

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
fn test_spend_balance() {
    let mut deps = mock_dependencies_custom(&[]);
    let amount = Uint128::from(100u128);
    let payee = Addr::unchecked("payee");
    let asset = "uusd";

    let res = spend_balance(deps.as_mut().storage, &payee, asset.to_string(), amount).unwrap();
    assert_eq!(res, amount.clone());

    BALANCES
        .save(
            deps.as_mut().storage,
            (payee.clone(), asset.to_string()),
            &Uint128::from(50u128),
        )
        .unwrap();

    let res = spend_balance(deps.as_mut().storage, &payee, asset.to_string(), amount).unwrap();
    let post_balance = BALANCES
        .load(deps.as_ref().storage, (payee.clone(), asset.to_string()))
        .unwrap();
    assert_eq!(res, Uint128::from(50u128));
    assert_eq!(post_balance, Uint128::from(0u128));

    BALANCES
        .save(
            deps.as_mut().storage,
            (payee.clone(), asset.to_string()),
            &Uint128::from(150u128),
        )
        .unwrap();

    let res = spend_balance(deps.as_mut().storage, &payee, asset.to_string(), amount).unwrap();
    let post_balance = BALANCES
        .load(deps.as_ref().storage, (payee, asset.to_string()))
        .unwrap();
    assert_eq!(res, Uint128::zero());
    assert_eq!(post_balance, Uint128::from(50u128));
}

#[test]
fn test_pay_fee() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let payee = "payee";

    let msg = ExecuteMsg::PayFee {
        payee: Addr::unchecked(payee),
        action: MOCK_ACTION.to_string(),
    };

    // Paying fee without funds
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::InsufficientFunds {});

    BALANCES
        .save(
            deps.as_mut().storage,
            (Addr::unchecked(payee), "uusd".to_string()),
            &Uint128::from(10u128),
        )
        .unwrap();

    //Paying fee with funds
    let res = execute(deps.as_mut(), env, info, msg);
    assert!(res.is_ok());

    let balance = BALANCES
        .load(
            deps.as_ref().storage,
            (Addr::unchecked(payee), "uusd".to_string()),
        )
        .unwrap();
    assert_eq!(balance, Uint128::from(0u128));

    // Check publisher balance
    let publisher = Addr::unchecked(MOCK_ADO_PUBLISHER);
    let balance = BALANCES
        .load(deps.as_ref().storage, (publisher, "uusd".to_string()))
        .unwrap_or_default();
    assert_eq!(balance, Uint128::from(10u128));
}

// Tests payment for fees via the contract balance
#[test]
fn test_pay_fee_contract() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let payee = "payee";

    let msg = ExecuteMsg::PayFee {
        payee: Addr::unchecked(payee),
        action: MOCK_ACTION.to_string(),
    };

    BALANCES
        .save(
            deps.as_mut().storage,
            (info.sender.clone(), "uusd".to_string()),
            &Uint128::from(10u128),
        )
        .unwrap();

    let res = execute(deps.as_mut(), env, info.clone(), msg);
    assert!(res.is_ok());

    let balance = BALANCES
        .load(deps.as_ref().storage, (info.sender, "uusd".to_string()))
        .unwrap();
    assert_eq!(balance, Uint128::from(0u128));

    // Check publisher balance
    let publisher = Addr::unchecked(MOCK_ADO_PUBLISHER);
    let balance = BALANCES
        .load(deps.as_ref().storage, (publisher, "uusd".to_string()))
        .unwrap_or_default();
    assert_eq!(balance, Uint128::from(10u128));
}

#[test]
fn test_pay_fee_app() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let payee = "payee";

    let msg = ExecuteMsg::PayFee {
        payee: Addr::unchecked(payee),
        action: MOCK_ACTION.to_string(),
    };

    BALANCES
        .save(
            deps.as_mut().storage,
            (Addr::unchecked(MOCK_APP_CONTRACT), "uusd".to_string()),
            &Uint128::from(10u128),
        )
        .unwrap();

    let res = execute(deps.as_mut(), env, info.clone(), msg);
    assert!(res.is_ok());

    let balance = BALANCES
        .load(deps.as_ref().storage, (info.sender, "uusd".to_string()))
        .unwrap();
    assert_eq!(balance, Uint128::from(0u128));

    // Check publisher balance
    let publisher = Addr::unchecked(MOCK_ADO_PUBLISHER);
    let balance = BALANCES
        .load(deps.as_ref().storage, (publisher, "uusd".to_string()))
        .unwrap_or_default();
    assert_eq!(balance, Uint128::from(10u128));
}

// Tests payment of fees via fallthrough
#[test]
fn test_pay_fee_joint() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let payee = "payee";

    let msg = ExecuteMsg::PayFee {
        payee: Addr::unchecked(payee),
        action: MOCK_ACTION.to_string(),
    };

    // Contract balance
    BALANCES
        .save(
            deps.as_mut().storage,
            (info.sender.clone(), "uusd".to_string()),
            &Uint128::from(4u128),
        )
        .unwrap();
    // Payee balance
    BALANCES
        .save(
            deps.as_mut().storage,
            (Addr::unchecked(payee), "uusd".to_string()),
            &Uint128::from(3u128),
        )
        .unwrap();
    // App balance
    BALANCES
        .save(
            deps.as_mut().storage,
            (Addr::unchecked(MOCK_APP_CONTRACT), "uusd".to_string()),
            &Uint128::from(3u128),
        )
        .unwrap();

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    // Check contract balance
    let balance = BALANCES
        .load(
            deps.as_ref().storage,
            (info.sender.clone(), "uusd".to_string()),
        )
        .unwrap();
    assert_eq!(balance, Uint128::from(0u128));

    // Check payee balance
    let balance = BALANCES
        .load(
            deps.as_ref().storage,
            (Addr::unchecked(payee), "uusd".to_string()),
        )
        .unwrap();
    assert_eq!(balance, Uint128::from(0u128));

    // Check app balance
    let balance = BALANCES
        .load(
            deps.as_ref().storage,
            (Addr::unchecked(MOCK_APP_CONTRACT), "uusd".to_string()),
        )
        .unwrap();
    assert_eq!(balance, Uint128::from(0u128));

    // Check publisher balance
    let publisher = Addr::unchecked(MOCK_ADO_PUBLISHER);
    let balance = BALANCES
        .load(deps.as_ref().storage, (publisher, "uusd".to_string()))
        .unwrap_or_default();
    assert_eq!(balance, Uint128::from(10u128));

    // Check insufficient funds
    // Contract balance
    BALANCES
        .save(
            deps.as_mut().storage,
            (info.sender.clone(), "uusd".to_string()),
            &Uint128::from(4u128),
        )
        .unwrap();
    // App balance
    BALANCES
        .save(
            deps.as_mut().storage,
            (Addr::unchecked(MOCK_APP_CONTRACT), "uusd".to_string()),
            &Uint128::from(3u128),
        )
        .unwrap();

    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res, ContractError::InsufficientFunds {});
}

#[test]
fn test_withdraw() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let asset = "uusd";

    //Withdraw all funds
    let msg = ExecuteMsg::Withdraw {
        amount: None,
        asset: asset.to_string(),
    };

    BALANCES
        .save(
            deps.as_mut().storage,
            (info.sender.clone(), asset.to_string()),
            &Uint128::from(10u128),
        )
        .unwrap();

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages.first().unwrap().msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![coin(10, asset)],
        })
    );

    let balance = BALANCES
        .load(
            deps.as_ref().storage,
            (info.sender.clone(), "uusd".to_string()),
        )
        .unwrap();
    assert_eq!(balance, Uint128::from(0u128));

    //Insufficient balance
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(res, ContractError::InsufficientFunds {});

    let msg = ExecuteMsg::Withdraw {
        amount: Some(Uint128::from(10u128)),
        asset: asset.to_string(),
    };

    BALANCES
        .save(
            deps.as_mut().storage,
            (info.sender.clone(), asset.to_string()),
            &Uint128::from(1u128),
        )
        .unwrap();

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(res, ContractError::InsufficientFunds {});

    // Partial withdraw
    let msg = ExecuteMsg::Withdraw {
        amount: Some(Uint128::from(5u128)),
        asset: asset.to_string(),
    };

    BALANCES
        .save(
            deps.as_mut().storage,
            (info.sender.clone(), asset.to_string()),
            &Uint128::from(10u128),
        )
        .unwrap();

    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages.first().unwrap().msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![coin(5, asset)],
        })
    );

    let balance = BALANCES
        .load(deps.as_ref().storage, (info.sender, "uusd".to_string()))
        .unwrap();
    assert_eq!(balance, Uint128::from(5u128));
}

fn cw20_deposit_msg(
    sender: impl Into<String>,
    amount: Uint128,
    recipient: Option<AndrAddr>,
) -> ExecuteMsg {
    ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: sender.into(),
        amount,
        msg: to_json_binary(&Cw20HookMsg::Deposit { address: recipient }).unwrap(),
    })
}

#[test]
fn test_cw20_deposit() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let asset = "cw20asset";
    let info = mock_info(asset, &[]);
    let depositee = "depositee";
    let recipient = AndrAddr::from_string("recipient");

    // Send 0 amount
    let msg = cw20_deposit_msg(depositee, Uint128::zero(), None);

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(
        res,
        ContractError::InvalidFunds {
            msg: "Cannot send 0 amount to deposit".to_string()
        }
    );

    // Send valid amount direct deposit
    let msg = cw20_deposit_msg(depositee, Uint128::from(10u128), None);

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert!(res.is_ok());

    // Check sender balance
    let balance = BALANCES
        .load(
            deps.as_ref().storage,
            (Addr::unchecked(depositee), asset.to_string()),
        )
        .unwrap();
    assert_eq!(balance, Uint128::from(10u128));

    // Send valid amount deposit on behalf
    let msg = cw20_deposit_msg(depositee, Uint128::from(10u128), Some(recipient.clone()));

    let res = execute(deps.as_mut(), env, info, msg);
    assert!(res.is_ok());

    let balance = BALANCES
        .load(
            deps.as_ref().storage,
            (Addr::unchecked(recipient.to_string()), asset.to_string()),
        )
        .unwrap();
    assert_eq!(balance, Uint128::from(10u128));
}

#[test]
fn test_withdraw_cw20() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let asset = "uusd";

    //Withdraw all funds
    let msg = ExecuteMsg::WithdrawCW20 {
        amount: None,
        asset: asset.to_string(),
    };

    BALANCES
        .save(
            deps.as_mut().storage,
            (info.sender.clone(), asset.to_string()),
            &Uint128::from(10u128),
        )
        .unwrap();

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages.first().unwrap().msg,
        cw20_withdraw_msg(
            Uint128::from(10u128),
            asset.to_string(),
            info.sender.clone()
        )
        .msg
    );

    let balance = BALANCES
        .load(
            deps.as_ref().storage,
            (info.sender.clone(), "uusd".to_string()),
        )
        .unwrap();
    assert_eq!(balance, Uint128::from(0u128));

    //Insufficient balance
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(res, ContractError::InsufficientFunds {});

    let msg = ExecuteMsg::WithdrawCW20 {
        amount: Some(Uint128::from(10u128)),
        asset: asset.to_string(),
    };

    BALANCES
        .save(
            deps.as_mut().storage,
            (info.sender.clone(), asset.to_string()),
            &Uint128::from(1u128),
        )
        .unwrap();

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(res, ContractError::InsufficientFunds {});

    // Partial withdraw
    let msg = ExecuteMsg::WithdrawCW20 {
        amount: Some(Uint128::from(5u128)),
        asset: asset.to_string(),
    };

    BALANCES
        .save(
            deps.as_mut().storage,
            (info.sender.clone(), asset.to_string()),
            &Uint128::from(10u128),
        )
        .unwrap();

    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages.first().unwrap().msg,
        cw20_withdraw_msg(Uint128::from(5u128), asset, info.sender.clone()).msg
    );

    let balance = BALANCES
        .load(deps.as_ref().storage, (info.sender, "uusd".to_string()))
        .unwrap();
    assert_eq!(balance, Uint128::from(5u128));
}
