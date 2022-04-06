use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    coins,
    testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR},
    to_binary, Addr, DepsMut, Response, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::{
    contract::{execute, instantiate},
    state::{
        Config, GlobalRewardInfo, Staker, State, CONFIG, GLOBAL_REWARD_INFOS, STAKERS,
        STAKER_REWARD_INFOS, STATE,
    },
    testing::mock_querier::mock_dependencies_custom,
};
use andromeda_protocol::cw20_staking::{Cw20HookMsg, ExecuteMsg, InstantiateMsg};
use common::{error::ContractError, mission::AndrAddress};
use cw_asset::{AssetInfo, AssetInfoUnchecked};

const MOCK_STAKING_TOKEN: &str = "staking_token";
const MOCK_INCENTIVE_TOKEN: &str = "incentive_token";

fn init(deps: DepsMut, additional_rewards: Option<Vec<AssetInfoUnchecked>>) -> Response {
    let info = mock_info("owner", &[]);

    let msg = InstantiateMsg {
        staking_token: AndrAddress {
            identifier: MOCK_STAKING_TOKEN.to_owned(),
        },
        additional_rewards,
    };

    instantiate(deps, mock_env(), info, msg).unwrap()
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies(&[]);

    let res = init(
        deps.as_mut(),
        Some(vec![
            AssetInfoUnchecked::native("uusd"),
            AssetInfoUnchecked::cw20("incentive_token"),
        ]),
    );

    assert_eq!(
        Response::new()
            .add_attribute("method", "instantiate")
            .add_attribute("type", "cw20_staking"),
        res
    );

    assert_eq!(
        Config {
            staking_token: AndrAddress {
                identifier: MOCK_STAKING_TOKEN.to_owned()
            },
            additional_reward_tokens: vec![
                AssetInfo::native("uusd"),
                AssetInfo::cw20(Addr::unchecked("incentive_token"))
            ]
        },
        CONFIG.load(deps.as_ref().storage).unwrap()
    );

    assert_eq!(
        GlobalRewardInfo::default(),
        GLOBAL_REWARD_INFOS
            .load(deps.as_ref().storage, "native:uusd")
            .unwrap()
    );
    assert_eq!(
        GlobalRewardInfo::default(),
        GLOBAL_REWARD_INFOS
            .load(deps.as_ref().storage, "cw20:incentive_token")
            .unwrap()
    );

    assert_eq!(
        State {
            total_share: Uint128::zero(),
        },
        STATE.load(deps.as_ref().storage).unwrap()
    );
}

#[test]
fn test_stake_unstake_tokens() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    deps.querier.with_token_balances(&[(
        &MOCK_STAKING_TOKEN.to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::new(100))],
    )]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "sender".to_string(),
        amount: Uint128::new(100),
        msg: to_binary(&Cw20HookMsg::StakeTokens {}).unwrap(),
    });

    let info = mock_info(MOCK_STAKING_TOKEN, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "stake_tokens")
            .add_attribute("sender", "sender")
            .add_attribute("share", "100")
            .add_attribute("amount", "100"),
        res
    );

    assert_eq!(
        State {
            total_share: Uint128::new(100)
        },
        STATE.load(deps.as_ref().storage).unwrap()
    );

    assert_eq!(
        Staker {
            share: Uint128::new(100)
        },
        STAKERS.load(deps.as_ref().storage, "sender").unwrap()
    );

    deps.querier.with_token_balances(&[(
        &MOCK_STAKING_TOKEN.to_string(),
        // 200 from last time and 100 for the deposit made by other_sender
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::new(200 + 100))],
    )]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "other_sender".to_string(),
        amount: Uint128::new(100),
        msg: to_binary(&Cw20HookMsg::StakeTokens {}).unwrap(),
    });

    let info = mock_info(MOCK_STAKING_TOKEN, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "stake_tokens")
            .add_attribute("sender", "other_sender")
            .add_attribute("share", "50")
            .add_attribute("amount", "100"),
        res
    );

    assert_eq!(
        State {
            total_share: Uint128::new(150)
        },
        STATE.load(deps.as_ref().storage).unwrap()
    );

    assert_eq!(
        Staker {
            share: Uint128::new(50)
        },
        STAKERS.load(deps.as_ref().storage, "other_sender").unwrap()
    );

    // User 1 tries to unstake too many tokens.
    let msg = ExecuteMsg::UnstakeTokens {
        amount: Some(Uint128::new(202)),
    };

    let info = mock_info("sender", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());

    assert_eq!(
        ContractError::InvalidWithdrawal {
            msg: Some("Desired amount exceeds balance".to_string()),
        },
        res.unwrap_err()
    );

    // User 1 unstakes all
    let msg = ExecuteMsg::UnstakeTokens { amount: None };

    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "unstake_tokens")
            .add_attribute("sender", "sender")
            .add_attribute("withdraw_amount", "200")
            .add_attribute("withdraw_share", "100")
            .add_message(WasmMsg::Execute {
                contract_addr: MOCK_STAKING_TOKEN.to_owned(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "sender".to_string(),
                    amount: Uint128::new(200)
                })
                .unwrap()
            }),
        res
    );

    assert_eq!(
        State {
            total_share: Uint128::new(50)
        },
        STATE.load(deps.as_ref().storage).unwrap()
    );

    assert_eq!(
        Staker {
            share: Uint128::zero()
        },
        STAKERS.load(deps.as_ref().storage, "sender").unwrap()
    );

    deps.querier.with_token_balances(&[(
        &MOCK_STAKING_TOKEN.to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::new(100))],
    )]);

    // User 2 unstakes all
    let msg = ExecuteMsg::UnstakeTokens { amount: None };

    let info = mock_info("other_sender", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "unstake_tokens")
            .add_attribute("sender", "other_sender")
            .add_attribute("withdraw_amount", "100")
            .add_attribute("withdraw_share", "50")
            .add_message(WasmMsg::Execute {
                contract_addr: MOCK_STAKING_TOKEN.to_owned(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "other_sender".to_string(),
                    amount: Uint128::new(100)
                })
                .unwrap()
            }),
        res
    );

    assert_eq!(
        State {
            total_share: Uint128::zero()
        },
        STATE.load(deps.as_ref().storage).unwrap()
    );

    assert_eq!(
        Staker {
            share: Uint128::zero()
        },
        STAKERS.load(deps.as_ref().storage, "other_sender").unwrap()
    );
}

#[test]
fn test_stake_invalid_token() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "sender".to_string(),
        amount: Uint128::new(100),
        msg: to_binary(&Cw20HookMsg::StakeTokens {}).unwrap(),
    });

    let info = mock_info("invalid_token", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidFunds {
            msg: "Deposited cw20 token is not the staking token".to_string(),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_update_global_indexes() {
    let mut deps = mock_dependencies_custom(&coins(40, "uusd"));
    init(
        deps.as_mut(),
        Some(vec![
            AssetInfoUnchecked::native("uusd"),
            AssetInfoUnchecked::cw20(MOCK_INCENTIVE_TOKEN),
        ]),
    );

    deps.querier.with_token_balances(&[
        (
            &MOCK_STAKING_TOKEN.to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::new(100))],
        ),
        (
            &MOCK_INCENTIVE_TOKEN.to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::new(20))],
        ),
    ]);

    STATE
        .save(
            deps.as_mut().storage,
            &State {
                total_share: Uint128::new(100),
            },
        )
        .unwrap();

    let msg = ExecuteMsg::UpdateGlobalIndexes { asset_infos: None };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new().add_attribute("action", "update_global_indexes"),
        res
    );

    assert_eq!(
        GlobalRewardInfo {
            index: Decimal256::from_ratio(Uint256::from(40u128), Uint256::from(100u128)),
            previous_reward_balance: Uint128::new(40)
        },
        GLOBAL_REWARD_INFOS
            .load(deps.as_ref().storage, "native:uusd")
            .unwrap()
    );

    assert_eq!(
        GlobalRewardInfo {
            index: Decimal256::from_ratio(Uint256::from(20u128), Uint256::from(100u128)),
            previous_reward_balance: Uint128::new(20)
        },
        GLOBAL_REWARD_INFOS
            .load(deps.as_ref().storage, "cw20:incentive_token")
            .unwrap()
    );
}

#[test]
fn test_update_global_indexes_selective() {
    let mut deps = mock_dependencies_custom(&coins(40, "uusd"));
    init(
        deps.as_mut(),
        Some(vec![
            AssetInfoUnchecked::native("uusd"),
            AssetInfoUnchecked::cw20(MOCK_INCENTIVE_TOKEN),
        ]),
    );

    deps.querier.with_token_balances(&[
        (
            &MOCK_STAKING_TOKEN.to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::new(100))],
        ),
        (
            &MOCK_INCENTIVE_TOKEN.to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::new(20))],
        ),
    ]);

    STATE
        .save(
            deps.as_mut().storage,
            &State {
                total_share: Uint128::new(100),
            },
        )
        .unwrap();

    let msg = ExecuteMsg::UpdateGlobalIndexes {
        asset_infos: Some(vec![AssetInfoUnchecked::native("uusd")]),
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new().add_attribute("action", "update_global_indexes"),
        res
    );

    assert_eq!(
        GlobalRewardInfo {
            index: Decimal256::from_ratio(Uint256::from(40u128), Uint256::from(100u128)),
            previous_reward_balance: Uint128::new(40)
        },
        GLOBAL_REWARD_INFOS
            .load(deps.as_ref().storage, "native:uusd")
            .unwrap()
    );

    assert_eq!(
        GlobalRewardInfo {
            index: Decimal256::zero(),
            previous_reward_balance: Uint128::zero()
        },
        GLOBAL_REWARD_INFOS
            .load(deps.as_ref().storage, "cw20:incentive_token")
            .unwrap()
    );
}

#[test]
fn test_update_global_indexes_invalid_asset() {
    let mut deps = mock_dependencies_custom(&coins(40, "uusd"));
    init(
        deps.as_mut(),
        Some(vec![
            AssetInfoUnchecked::native("uusd"),
            AssetInfoUnchecked::cw20(MOCK_INCENTIVE_TOKEN),
        ]),
    );

    STATE
        .save(
            deps.as_mut().storage,
            &State {
                total_share: Uint128::new(100),
            },
        )
        .unwrap();

    let msg = ExecuteMsg::UpdateGlobalIndexes {
        asset_infos: Some(vec![AssetInfoUnchecked::native("uluna")]),
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidAsset {
            asset: "native:uluna".to_string(),
        },
        res.unwrap_err()
    );
}
