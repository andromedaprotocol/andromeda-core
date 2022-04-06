use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, DepsMut, Response, Uint128,
};

use crate::{
    contract::{execute, instantiate},
    state::{
        Config, GlobalRewardInfo, State, CONFIG, GLOBAL_REWARD_INFOS, STAKER_REWARD_INFOS, STATE,
    },
    testing::mock_querier::{mock_dependencies_custom, MOCK_STAKING_TOKEN},
};
use andromeda_protocol::cw20_staking::{ExecuteMsg, InstantiateMsg};
use common::mission::AndrAddress;
use cw_asset::{AssetInfo, AssetInfoUnchecked};

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
fn test_stake_tokens() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);
}
