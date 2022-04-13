use cosmwasm_std::{
    coin, coins,
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, BankMsg, DepsMut, Response, Uint128,
};

use crate::{
    contract::{execute, instantiate, query},
    state::{Config, State, UserInfo, CONFIG, STATE, USER_INFO},
};
use andromeda_protocol::lockdrop::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, StateResponse,
    UserInfoResponse,
};
use common::{error::ContractError, mission::AndrAddress};
use cw20::Cw20ReceiveMsg;

const MOCK_INCENTIVE_TOKEN: &str = "mock_incentive_token";
const MOCK_AUCTION_CONTRACT: &str = "mock_auction_contract";
const DEPOSIT_WINDOW: u64 = 5;
const WITHDRAWAL_WINDOW: u64 = 4;

fn init(deps: DepsMut) -> Result<Response, ContractError> {
    let env = mock_env();
    let info = mock_info("owner", &[]);

    let msg = InstantiateMsg {
        auction_contract: None,
        init_timestamp: env.block.time.seconds(),
        deposit_window: DEPOSIT_WINDOW,
        withdrawal_window: WITHDRAWAL_WINDOW,
        incentive_token: MOCK_INCENTIVE_TOKEN.to_owned(),
        native_denom: "uusd".to_string(),
    };

    instantiate(deps, env, info, msg)
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies(&[]);

    let res = init(deps.as_mut()).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("method", "instantiate")
            .add_attribute("type", "lockdrop"),
        res
    );

    assert_eq!(
        Config {
            auction_contract_address: None,
            init_timestamp: mock_env().block.time.seconds(),
            deposit_window: DEPOSIT_WINDOW,
            withdrawal_window: WITHDRAWAL_WINDOW,
            lockdrop_incentives: Uint128::zero(),
            incentive_token: MOCK_INCENTIVE_TOKEN.to_owned(),
            native_denom: "uusd".to_string()
        },
        CONFIG.load(deps.as_ref().storage).unwrap()
    );

    assert_eq!(
        State {
            total_native_locked: Uint128::zero(),
            total_delegated: Uint128::zero(),
            are_claims_allowed: false,
        },
        STATE.load(deps.as_ref().storage).unwrap()
    );
}

#[test]
fn test_instantiate_init_timestamp_past() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let info = mock_info("owner", &[]);

    let msg = InstantiateMsg {
        auction_contract: None,
        init_timestamp: env.block.time.seconds() - 1,
        deposit_window: 5,
        withdrawal_window: 2,
        incentive_token: MOCK_INCENTIVE_TOKEN.to_owned(),
        native_denom: "uusd".to_string(),
    };

    let res = instantiate(deps.as_mut(), env, info, msg);

    assert_eq!(ContractError::StartTimeInThePast {}, res.unwrap_err());
}

#[test]
fn test_instantiate_init_deposit_window_zero() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let info = mock_info("owner", &[]);

    let msg = InstantiateMsg {
        auction_contract: None,
        init_timestamp: env.block.time.seconds() + 1,
        deposit_window: 0,
        withdrawal_window: 2,
        incentive_token: MOCK_INCENTIVE_TOKEN.to_owned(),
        native_denom: "uusd".to_string(),
    };

    let res = instantiate(deps.as_mut(), env, info, msg);

    assert_eq!(ContractError::InvalidWindow {}, res.unwrap_err());
}

#[test]
fn test_instantiate_init_withdrawal_window_zero() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let info = mock_info("owner", &[]);

    let msg = InstantiateMsg {
        auction_contract: None,
        init_timestamp: env.block.time.seconds() + 1,
        deposit_window: 5,
        withdrawal_window: 0,
        incentive_token: MOCK_INCENTIVE_TOKEN.to_owned(),
        native_denom: "uusd".to_string(),
    };

    let res = instantiate(deps.as_mut(), env, info, msg);

    assert_eq!(ContractError::InvalidWindow {}, res.unwrap_err());
}

#[test]
fn test_instantiate_init_deposit_window_less_than_withdrawal_window() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let info = mock_info("owner", &[]);

    let msg = InstantiateMsg {
        auction_contract: None,
        init_timestamp: env.block.time.seconds() + 1,
        deposit_window: 2,
        withdrawal_window: 5,
        incentive_token: MOCK_INCENTIVE_TOKEN.to_owned(),
        native_denom: "uusd".to_string(),
    };

    let res = instantiate(deps.as_mut(), env, info, msg);

    assert_eq!(ContractError::InvalidWindow {}, res.unwrap_err());
}

#[test]
fn test_increase_incentives() {
    let mut deps = mock_dependencies(&[]);

    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "owner".to_string(),
        amount: Uint128::new(100),
        msg: to_binary(&Cw20HookMsg::IncreaseIncentives {}).unwrap(),
    });

    let info = mock_info(MOCK_INCENTIVE_TOKEN, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "incentives_increased")
            .add_attribute("amount", "100"),
        res
    );

    assert_eq!(
        Uint128::new(100),
        CONFIG
            .load(deps.as_ref().storage)
            .unwrap()
            .lockdrop_incentives
    );
}

#[test]
fn test_increase_incentives_invalid_token() {
    let mut deps = mock_dependencies(&[]);

    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "owner".to_string(),
        amount: Uint128::new(100),
        msg: to_binary(&Cw20HookMsg::IncreaseIncentives {}).unwrap(),
    });

    let info = mock_info("invalid_token", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(
        ContractError::InvalidFunds {
            msg: "Only incentive tokens are valid".to_string(),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_increase_incentives_after_phase_ends() {
    let mut deps = mock_dependencies(&[]);
    let mut env = mock_env();

    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "owner".to_string(),
        amount: Uint128::new(100),
        msg: to_binary(&Cw20HookMsg::IncreaseIncentives {}).unwrap(),
    });

    env.block.time = env
        .block
        .time
        .plus_seconds(DEPOSIT_WINDOW + WITHDRAWAL_WINDOW + 1);
    let info = mock_info(MOCK_INCENTIVE_TOKEN, &[]);
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(
        ContractError::TokenAlreadyBeingDistributed {},
        res.unwrap_err()
    );
}

#[test]
fn test_increase_incentives_zero_amount() {
    let mut deps = mock_dependencies(&[]);

    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "owner".to_string(),
        amount: Uint128::zero(),
        msg: to_binary(&Cw20HookMsg::IncreaseIncentives {}).unwrap(),
    });

    let info = mock_info(MOCK_INCENTIVE_TOKEN, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidFunds {
            msg: "Number of tokens should be > 0".to_string(),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_deposit_native() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::DepositNative {};
    let info = mock_info("sender", &coins(100, "uusd"));

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "lock_native")
            .add_attribute("user", "sender")
            .add_attribute("ust_deposited", "100"),
        res
    );

    assert_eq!(
        State {
            total_native_locked: Uint128::new(100),
            total_delegated: Uint128::zero(),
            are_claims_allowed: false
        },
        STATE.load(deps.as_ref().storage,).unwrap()
    );

    assert_eq!(
        UserInfo {
            total_native_locked: Uint128::new(100),
            delegated_incentives: Uint128::zero(),
            lockdrop_claimed: false,
            withdrawal_flag: false,
        },
        USER_INFO
            .load(deps.as_ref().storage, &Addr::unchecked("sender"))
            .unwrap()
    );
}

#[test]
fn test_deposit_native_zero_amount() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::DepositNative {};
    let info = mock_info("sender", &coins(0, "uusd"));

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidFunds {
            msg: "Amount must be greater than 0".to_string(),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_deposit_native_wrong_denom() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::DepositNative {};
    let info = mock_info("sender", &coins(100, "uluna"));

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidFunds {
            msg: "Only uusd accepted".to_string(),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_deposit_native_multiple_denoms() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::DepositNative {};
    let info = mock_info("sender", &[coin(100, "uluna"), coin(100, "uusd")]);

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidFunds {
            msg: "Must deposit a single fund".to_string(),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_deposit_native_deposit_window_closed() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::DepositNative {};
    let info = mock_info("sender", &coins(100, "uusd"));

    let mut env = mock_env();
    env.block.time = env.block.time.plus_seconds(DEPOSIT_WINDOW + 1);
    let res = execute(deps.as_mut(), env, info, msg);

    assert_eq!(ContractError::DepositWindowClosed {}, res.unwrap_err());
}

#[test]
fn test_withdraw_native() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::DepositNative {};
    let info = mock_info("sender", &coins(100, "uusd"));

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::WithdrawNative { amount: None };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "sender".to_string(),
                amount: coins(100, "uusd")
            })
            .add_attribute("action", "withdraw_native")
            .add_attribute("user", "sender")
            .add_attribute("amount", "100"),
        res
    );

    assert_eq!(
        State {
            total_native_locked: Uint128::zero(),
            total_delegated: Uint128::zero(),
            are_claims_allowed: false
        },
        STATE.load(deps.as_ref().storage,).unwrap()
    );

    assert_eq!(
        UserInfo {
            total_native_locked: Uint128::zero(),
            delegated_incentives: Uint128::zero(),
            lockdrop_claimed: false,
            withdrawal_flag: false,
        },
        USER_INFO
            .load(deps.as_ref().storage, &Addr::unchecked("sender"))
            .unwrap()
    );
}

#[test]
fn test_withdraw_native_withdraw_phase_first_half() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::DepositNative {};
    let info = mock_info("sender", &coins(100, "uusd"));

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::WithdrawNative {
        amount: Some(Uint128::new(51)),
    };

    let mut env = mock_env();
    env.block.time = env.block.time.plus_seconds(DEPOSIT_WINDOW + 1);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);

    assert_eq!(
        ContractError::InvalidWithdrawal {
            msg: Some("Amount exceeds max allowed withdrawal limit of 50".to_string()),
        },
        res.unwrap_err()
    );

    let msg = ExecuteMsg::WithdrawNative { amount: None };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "sender".to_string(),
                amount: coins(50, "uusd")
            })
            .add_attribute("action", "withdraw_native")
            .add_attribute("user", "sender")
            // Only half is withdrawable in the first half of the withdrawal period
            .add_attribute("amount", "50"),
        res
    );

    assert_eq!(
        State {
            total_native_locked: Uint128::new(50),
            total_delegated: Uint128::zero(),
            are_claims_allowed: false
        },
        STATE.load(deps.as_ref().storage,).unwrap()
    );

    assert_eq!(
        UserInfo {
            total_native_locked: Uint128::new(50),
            delegated_incentives: Uint128::zero(),
            lockdrop_claimed: false,
            withdrawal_flag: true,
        },
        USER_INFO
            .load(deps.as_ref().storage, &Addr::unchecked("sender"))
            .unwrap()
    );
}

#[test]
fn test_withdraw_native_withdraw_phase_second_half() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::DepositNative {};
    let info = mock_info("sender", &coins(100, "uusd"));

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::WithdrawNative { amount: None };

    let mut env = mock_env();
    env.block.time = env
        .block
        .time
        .plus_seconds(DEPOSIT_WINDOW + 3 * WITHDRAWAL_WINDOW / 4);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "sender".to_string(),
                amount: coins(25, "uusd")
            })
            .add_attribute("action", "withdraw_native")
            .add_attribute("user", "sender")
            // In second half of withdrawal phase, percent decreases linearly from 50% to 0%.
            .add_attribute("amount", "25"),
        res
    );

    assert_eq!(
        State {
            total_native_locked: Uint128::new(75),
            total_delegated: Uint128::zero(),
            are_claims_allowed: false
        },
        STATE.load(deps.as_ref().storage).unwrap()
    );

    assert_eq!(
        UserInfo {
            total_native_locked: Uint128::new(75),
            delegated_incentives: Uint128::zero(),
            lockdrop_claimed: false,
            withdrawal_flag: true,
        },
        USER_INFO
            .load(deps.as_ref().storage, &Addr::unchecked("sender"))
            .unwrap()
    );

    // try to withdraw again
    let res = execute(deps.as_mut(), env, info, msg);

    assert_eq!(
        ContractError::InvalidWithdrawal {
            msg: Some("Max 1 withdrawal allowed".to_string()),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_withdraw_native_withdrawal_closed() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::DepositNative {};
    let info = mock_info("sender", &coins(100, "uusd"));

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::WithdrawNative { amount: None };

    let mut env = mock_env();
    env.block.time = env
        .block
        .time
        .plus_seconds(DEPOSIT_WINDOW + WITHDRAWAL_WINDOW + 1);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());

    assert_eq!(
        ContractError::InvalidWithdrawal {
            msg: Some("Withdrawals not available".to_string()),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_withdraw_proceeds_unauthorized() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::WithdrawProceeds { recipient: None };

    let info = mock_info("not owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_withdraw_proceeds_phase_not_started() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::WithdrawProceeds { recipient: None };

    let info = mock_info("owner", &[]);
    let mut env = mock_env();
    env.block.time = env.block.time.minus_seconds(1);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidWithdrawal {
            msg: Some("Lockdrop withdrawals haven't concluded yet".to_string()),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_withdraw_proceeds_phase_not_ended() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::WithdrawProceeds { recipient: None };

    let info = mock_info("owner", &[]);
    let mut env = mock_env();
    env.block.time = env.block.time.plus_seconds(DEPOSIT_WINDOW);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidWithdrawal {
            msg: Some("Lockdrop withdrawals haven't concluded yet".to_string()),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_withdraw_proceeds() {
    // This uusd is to simulate the deposit made prior to withdrawing proceeds. This is needed
    // since the mock querier doesn't automatically assign balances.
    let amount = 100;
    let mut deps = mock_dependencies(&coins(amount, "uusd"));
    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::DepositNative {};
    let info = mock_info("sender", &coins(amount, "uusd"));

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::WithdrawProceeds { recipient: None };

    let info = mock_info("owner", &[]);
    let mut env = mock_env();
    env.block.time = env
        .block
        .time
        .plus_seconds(DEPOSIT_WINDOW + WITHDRAWAL_WINDOW + 1);

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "owner".to_string(),
                amount: coins(100, "uusd")
            })
            .add_attribute("action", "withdraw_proceeds")
            .add_attribute("amount", "100")
            .add_attribute("timestamp", env.block.time.seconds().to_string()),
        res
    );

    // Remove withdrawn funds.
    deps.querier
        .update_balance(env.contract.address.clone(), vec![]);

    // try to withdraw again
    let res = execute(deps.as_mut(), env.clone(), info, msg);

    assert_eq!(
        ContractError::InvalidWithdrawal {
            msg: Some("Already withdrew funds".to_string()),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_enable_claims_no_auction_specified() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::EnableClaims {};

    let mut env = mock_env();
    env.block.time = env
        .block
        .time
        .plus_seconds(DEPOSIT_WINDOW + WITHDRAWAL_WINDOW + 1);

    let info = mock_info("sender", &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        Response::new().add_attribute("action", "enable_claims"),
        res
    );

    assert_eq!(
        State {
            total_delegated: Uint128::zero(),
            total_native_locked: Uint128::zero(),
            are_claims_allowed: true
        },
        STATE.load(deps.as_ref().storage).unwrap()
    );

    // Try to do it again.
    let res = execute(deps.as_mut(), env, info, msg);

    assert_eq!(ContractError::ClaimsAlreadyAllowed {}, res.unwrap_err());
}

#[test]
fn test_enable_claims_auction_specified() {
    let mut deps = mock_dependencies(&[]);
    let msg = InstantiateMsg {
        auction_contract: Some(AndrAddress {
            identifier: MOCK_AUCTION_CONTRACT.to_owned(),
        }),
        init_timestamp: mock_env().block.time.seconds(),
        deposit_window: DEPOSIT_WINDOW,
        withdrawal_window: WITHDRAWAL_WINDOW,
        incentive_token: MOCK_INCENTIVE_TOKEN.to_owned(),
        native_denom: "uusd".to_string(),
    };

    let info = mock_info("owner", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::EnableClaims {};

    let mut env = mock_env();
    env.block.time = env
        .block
        .time
        .plus_seconds(DEPOSIT_WINDOW + WITHDRAWAL_WINDOW + 1);

    let info = mock_info("not_auction_contract", &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

    let info = mock_info(MOCK_AUCTION_CONTRACT, &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        Response::new().add_attribute("action", "enable_claims"),
        res
    );

    assert_eq!(
        State {
            total_delegated: Uint128::zero(),
            total_native_locked: Uint128::zero(),
            are_claims_allowed: true
        },
        STATE.load(deps.as_ref().storage).unwrap()
    );

    // Try to do it again.
    let res = execute(deps.as_mut(), env, info, msg);

    assert_eq!(ContractError::ClaimsAlreadyAllowed {}, res.unwrap_err());
}

#[test]
fn test_enable_claims_phase_not_ended() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut()).unwrap();

    let msg = ExecuteMsg::EnableClaims {};

    let mut env = mock_env();
    env.block.time = env
        .block
        .time
        .plus_seconds(DEPOSIT_WINDOW + WITHDRAWAL_WINDOW);

    let info = mock_info("sender", &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());

    assert_eq!(ContractError::PhaseOngoing {}, res.unwrap_err());
}
