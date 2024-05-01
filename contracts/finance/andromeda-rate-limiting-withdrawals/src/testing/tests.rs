use andromeda_std::{ado_base::modules::Module, common::Milliseconds, error::ContractError};
use cosmwasm_std::{
    coin,
    testing::{mock_env, mock_info},
    DepsMut, Response, Uint128,
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

    let exec = ExecuteMsg::Deposits { recipient: None };
    let _err = execute(deps.as_mut(), mock_env(), info, exec).unwrap_err();
}

#[test]
fn test_deposit_invalid_funds() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut());

    let exec = ExecuteMsg::Deposits {
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

    let exec = ExecuteMsg::Deposits {
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

    let exec = ExecuteMsg::Deposits {
        recipient: Some("andromedauser".to_string()),
    };

    let info = mock_info("creator", &[coin(30, "junox")]);

    let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();
    let exec = ExecuteMsg::Deposits { recipient: None };

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

    let exec = ExecuteMsg::Deposits {
        recipient: Some("andromedauser".to_string()),
    };

    let info = mock_info("creator", &[coin(30, "junox")]);

    let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

    let info = mock_info("random", &[]);
    let exec = ExecuteMsg::WithdrawFunds {
        amount: Uint128::from(19_u16),
    };
    let err = execute(deps.as_mut(), mock_env(), info, exec).unwrap_err();
    assert_eq!(err, ContractError::AccountNotFound {});
}

#[test]
fn test_withdraw_over_account_limit() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut());

    let exec = ExecuteMsg::Deposits {
        recipient: Some("andromedauser".to_string()),
    };

    let info = mock_info("creator", &[coin(30, "junox")]);

    let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

    let info = mock_info("andromedauser", &[]);
    let exec = ExecuteMsg::WithdrawFunds {
        amount: Uint128::from(31_u16),
    };
    let err = execute(deps.as_mut(), mock_env(), info, exec).unwrap_err();
    assert_eq!(err, ContractError::InsufficientFunds {});
}

#[test]
fn test_withdraw_funds_locked() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut());

    let exec = ExecuteMsg::Deposits {
        recipient: Some("andromedauser".to_string()),
    };

    let info = mock_info("creator", &[coin(30, "junox")]);

    let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

    let info = mock_info("andromedauser", &[]);
    let exec = ExecuteMsg::WithdrawFunds {
        amount: Uint128::from(10_u16),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

    let info = mock_info("andromedauser", &[]);
    let exec = ExecuteMsg::WithdrawFunds {
        amount: Uint128::from(10_u16),
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
    let exec = ExecuteMsg::Deposits {
        recipient: Some("andromedauser".to_string()),
    };

    let info = mock_info("creator", &[coin(30, "junox")]);

    let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

    let info = mock_info("andromedauser", &[]);
    let exec = ExecuteMsg::WithdrawFunds {
        amount: Uint128::from(21_u16),
    };
    let err = execute(deps.as_mut(), mock_env(), info, exec).unwrap_err();
    assert_eq!(err, ContractError::WithdrawalLimitExceeded {});
}

#[test]
fn test_withdraw_works() {
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
    let exec = ExecuteMsg::Deposits {
        recipient: Some("andromedauser".to_string()),
    };

    let info = mock_info("creator", &[coin(30, "junox")]);

    let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

    let info = mock_info("andromedauser", &[]);
    let exec = ExecuteMsg::WithdrawFunds {
        amount: Uint128::from(10_u16),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

    let expected_balance = AccountDetails {
        balance: Uint128::from(20_u16),
        latest_withdrawal: Some(env.block.time),
    };
    let actual_balance = ACCOUNTS
        .load(&deps.storage, "andromedauser".to_string())
        .unwrap();
    assert_eq!(expected_balance, actual_balance)
}
