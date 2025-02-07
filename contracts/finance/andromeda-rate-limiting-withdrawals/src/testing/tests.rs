use andromeda_std::{
    amp::{AndrAddr, Recipient},
    common::Milliseconds,
    error::ContractError,
};
use cosmwasm_std::{
    coin, from_json,
    testing::{mock_env, mock_info},
    BankMsg, Binary, CosmosMsg, DepsMut, Response, Uint128, WasmMsg,
};
pub const OWNER: &str = "creator";

use super::mock_querier::MOCK_KERNEL_CONTRACT;

use crate::{
    contract::{execute, instantiate},
    state::ACCOUNTS,
    testing::mock_querier::mock_dependencies_custom,
};
use andromeda_finance::rate_limiting_withdrawals::{
    AccountDetails, CoinAndLimit, ExecuteMsg, InstantiateMsg, MinimumFrequency,
};

use rstest::*;

fn init(deps: DepsMut) -> Response {
    let msg = InstantiateMsg {
        owner: Some(OWNER.to_owned()),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        allowed_coin: CoinAndLimit {
            coin: "junox".to_string(),
            limit: Uint128::from(50_u64),
        },
        minimal_withdrawal_frequency: MinimumFrequency::Time {
            time: Milliseconds::from_seconds(10),
        },
    };

    let info = mock_info("owner", &[]);
    instantiate(deps, mock_env(), info, msg).unwrap()
}

#[test]
fn test_instantiate_works() {
    let mut deps = mock_dependencies_custom(&[]);
    let res = init(deps.as_mut());
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_deposit_zero_funds() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let _res = init(deps.as_mut());

    let exec = ExecuteMsg::Deposit { recipient: None };
    let _err = execute(deps.as_mut(), mock_env(), info, exec).unwrap_err();
}

#[test]
fn test_deposit_invalid_funds() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut());

    let exec = ExecuteMsg::Deposit {
        recipient: Some("me".to_string()),
    };

    let info = mock_info("creator", &[coin(30, "uusd")]);

    let err = execute(deps.as_mut(), mock_env(), info, exec).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidFunds {
            msg: "Coin must be part of the allowed list".to_string(),
        }
    )
}

#[test]
fn test_deposit_new_account_works() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut());

    let exec = ExecuteMsg::Deposit {
        recipient: Some("andromedauser".to_string()),
    };

    let info = mock_info("creator", &[coin(30, "junox")]);

    let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();
    let expected_balance = AccountDetails {
        balance: Uint128::from(30_u16),
        latest_withdrawal: None,
    };
    let actual_balance = ACCOUNTS
        .load(&deps.storage, "andromedauser".to_string())
        .unwrap();
    assert_eq!(expected_balance, actual_balance)
}

#[test]
fn test_deposit_existing_account_works() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut());

    let exec = ExecuteMsg::Deposit {
        recipient: Some("andromedauser".to_string()),
    };

    let info = mock_info("creator", &[coin(30, "junox")]);

    let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();
    let exec = ExecuteMsg::Deposit { recipient: None };

    let info = mock_info("andromedauser", &[coin(70, "junox")]);

    let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();
    let expected_balance = AccountDetails {
        balance: Uint128::from(100_u16),
        latest_withdrawal: None,
    };
    let actual_balance = ACCOUNTS
        .load(&deps.storage, "andromedauser".to_string())
        .unwrap();
    assert_eq!(expected_balance, actual_balance)
}

#[test]
fn test_withdraw_account_not_found() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut());

    let exec = ExecuteMsg::Deposit {
        recipient: Some("andromedauser".to_string()),
    };

    let info = mock_info("creator", &[coin(30, "junox")]);

    let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

    let info = mock_info("random", &[]);
    let exec = ExecuteMsg::Withdraw {
        amount: Uint128::from(19_u16),
        recipient: None,
    };
    let err = execute(deps.as_mut(), mock_env(), info, exec).unwrap_err();
    assert_eq!(err, ContractError::AccountNotFound {});
}

#[test]
fn test_withdraw_over_account_limit() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut());

    let exec = ExecuteMsg::Deposit {
        recipient: Some("andromedauser".to_string()),
    };

    let info = mock_info("creator", &[coin(30, "junox")]);

    let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

    let info = mock_info("andromedauser", &[]);
    let exec = ExecuteMsg::Withdraw {
        amount: Uint128::from(31_u16),
        recipient: None,
    };
    let err = execute(deps.as_mut(), mock_env(), info, exec).unwrap_err();
    assert_eq!(err, ContractError::InsufficientFunds {});
}

#[test]
fn test_withdraw_funds_locked() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut());

    let exec = ExecuteMsg::Deposit {
        recipient: Some("andromedauser".to_string()),
    };

    let info = mock_info("creator", &[coin(30, "junox")]);

    let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

    let info = mock_info("andromedauser", &[]);
    let exec = ExecuteMsg::Withdraw {
        amount: Uint128::from(10_u16),
        recipient: None,
    };
    let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

    let info = mock_info("andromedauser", &[]);
    let exec = ExecuteMsg::Withdraw {
        amount: Uint128::from(10_u16),
        recipient: None,
    };

    let err = execute(deps.as_mut(), mock_env(), info, exec).unwrap_err();

    assert_eq!(err, ContractError::FundsAreLocked {});
}

#[test]
fn test_withdraw_over_allowed_limit() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        allowed_coin: CoinAndLimit {
            coin: "junox".to_string(),
            limit: Uint128::from(20_u64),
        },
        minimal_withdrawal_frequency: MinimumFrequency::Time {
            time: Milliseconds::from_seconds(10),
        },
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
    };
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    let exec = ExecuteMsg::Deposit {
        recipient: Some("andromedauser".to_string()),
    };

    let info = mock_info("creator", &[coin(30, "junox")]);

    let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

    let info = mock_info("andromedauser", &[]);
    let exec = ExecuteMsg::Withdraw {
        amount: Uint128::from(21_u16),
        recipient: None,
    };
    let err = execute(deps.as_mut(), mock_env(), info, exec).unwrap_err();
    assert_eq!(err, ContractError::WithdrawalLimitExceeded {});
}

#[rstest]
#[case::direct(None, "andromedauser")] // Withdraw to self
#[case::with_recipient(Some(Recipient::new("recipient".to_string(), Some(Binary::default()))), "recipient")] // Withdraw to different recipient
fn test_withdraw_works(#[case] recipient: Option<Recipient>, #[case] expected_recipient: &str) {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        allowed_coin: CoinAndLimit {
            coin: "junox".to_string(),
            limit: Uint128::from(50_u64),
        },
        minimal_withdrawal_frequency: MinimumFrequency::Time {
            time: Milliseconds::from_seconds(10),
        },
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
    };
    let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
    let exec = ExecuteMsg::Deposit {
        recipient: Some("andromedauser".to_string()),
    };

    let info = mock_info("creator", &[coin(30, "junox")]);
    let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

    let info = mock_info("andromedauser", &[]);
    let exec = ExecuteMsg::Withdraw {
        amount: Uint128::from(10_u16),
        recipient: recipient.clone(),
    };
    let res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();
    let sub_msg = res.messages[0].msg.clone();

    if recipient.is_some() {
        if let CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            funds,
            msg,
        }) = sub_msg
        {
            assert_eq!(contract_addr, MOCK_KERNEL_CONTRACT);
            assert_eq!(funds, vec![coin(10, "junox")]);
            let msg: ExecuteMsg = from_json(&msg).unwrap_or_else(|e| {
                panic!("Failed to deserialize pkt: {}", e);
            });

            if let ExecuteMsg::AMPReceive(pkt) = msg {
                let amp_msg = pkt.messages[0].clone();
                assert_eq!(amp_msg.recipient, AndrAddr::from_string(expected_recipient));
                assert_eq!(amp_msg.message, Binary::default());
            } else {
                panic!("Message is not a AMPReceive");
            }
        } else {
            panic!("SubMsg is not a WasmMsg::Execute");
        }
    } else if let CosmosMsg::Bank(BankMsg::Send { to_address, amount }) = sub_msg {
        assert_eq!(to_address, expected_recipient.to_string());
        assert_eq!(amount, vec![coin(10, "junox")]);
    } else {
        panic!("SubMsg is not a BankMsg::Send");
    }

    let expected_balance = AccountDetails {
        balance: Uint128::from(20_u16),
        latest_withdrawal: Some(env.block.time),
    };
    let actual_balance = ACCOUNTS
        .load(&deps.storage, "andromedauser".to_string())
        .unwrap();
    assert_eq!(expected_balance, actual_balance)
}
