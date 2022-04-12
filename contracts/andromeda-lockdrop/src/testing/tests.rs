use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Response, Uint128,
};

use crate::{
    contract::{execute, instantiate, query},
    state::{Config, State, CONFIG, STATE},
};
use andromeda_protocol::lockdrop::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, StateResponse,
    UserInfoResponse,
};
use common::error::ContractError;

const MOCK_INCENTIVE_TOKEN: &str = "mock_incentive_token";

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let info = mock_info("owner", &[]);

    let msg = InstantiateMsg {
        auction_contract: None,
        init_timestamp: env.block.time.seconds(),
        deposit_window: 5,
        withdrawal_window: 2,
        incentive_token: MOCK_INCENTIVE_TOKEN.to_owned(),
    };

    let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("method", "instantiate")
            .add_attribute("type", "lockdrop"),
        res
    );

    assert_eq!(
        Config {
            auction_contract_address: None,
            init_timestamp: env.block.time.seconds(),
            deposit_window: 5,
            withdrawal_window: 2,
            lockdrop_incentives: Uint128::zero(),
            incentive_token: MOCK_INCENTIVE_TOKEN.to_owned(),
        },
        CONFIG.load(deps.as_ref().storage).unwrap()
    );

    assert_eq!(
        State {
            total_ust_locked: Uint128::zero(),
            total_mars_delegated: Uint128::zero(),
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
    };

    let res = instantiate(deps.as_mut(), env, info, msg);

    assert_eq!(ContractError::InvalidWindow {}, res.unwrap_err());
}
