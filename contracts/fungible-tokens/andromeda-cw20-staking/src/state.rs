use cosmwasm_std::{Api, Decimal256, Env, Order, QuerierWrapper, Storage, Uint128};
use cw_storage_plus::{Bound, Item, Map};

use crate::contract::{get_pending_rewards, get_staking_token};
use andromeda_fungible_tokens::cw20_staking::{RewardToken, StakerResponse};
use common::{app::AndrAddress, error::ContractError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_REWARD_TOKENS: u32 = 10;

pub const CONFIG: Item<Config> = Item::new("config");
pub const STATE: Item<State> = Item::new("state");
pub const STAKERS: Map<&str, Staker> = Map::new("stakers");

/// Maps asset -> reward_info
pub const REWARD_TOKENS: Map<&str, RewardToken> = Map::new("reward_tokens");

/// Maps (staker, asset) -> reward_info
pub const STAKER_REWARD_INFOS: Map<(&str, &str), StakerRewardInfo> =
    Map::new("staker_reward_infos");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// The token accepted for staking.
    pub staking_token: AndrAddress,
    /// The current number of reward tokens, cannot exceed `MAX_REWARD_TOKENS`.
    pub number_of_reward_tokens: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    /// The total share of the staking token in the contract.
    pub total_share: Uint128,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Staker {
    /// Total staked share.
    pub share: Uint128,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StakerRewardInfo {
    /// The index of this particular reward.
    pub index: Decimal256,
    /// The pending rewards for this particular reward.
    pub pending_rewards: Decimal256,
}

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub(crate) fn get_stakers(
    storage: &dyn Storage,
    querier: &QuerierWrapper,
    api: &dyn Api,
    env: &Env,
    start_after: Option<&str>,
    limit: Option<u32>,
) -> Result<Vec<StakerResponse>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    STAKERS
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|elem| {
            let (address, staker) = elem?;
            let state = STATE.load(storage)?;
            let pending_rewards = get_pending_rewards(storage, querier, env, &address, &staker)?;
            let staking_token = get_staking_token(storage, api, querier)?;
            let total_balance =
                staking_token.query_balance(querier, env.contract.address.clone())?;
            let balance = staker
                .share
                .multiply_ratio(total_balance, state.total_share);
            Ok(StakerResponse {
                address,
                share: staker.share,
                pending_rewards,
                balance,
            })
        })
        .collect()
}
