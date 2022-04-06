use common::mission::AndrAddress;
use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::Uint128;
use cw_asset::AssetInfo;
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONFIG: Item<Config> = Item::new("config");
pub const STATE: Item<State> = Item::new("state");
pub const STAKERS: Map<&str, Staker> = Map::new("stakers");

/// Maps asset -> reward_info
pub const GLOBAL_REWARD_INFOS: Map<&str, GlobalRewardInfo> = Map::new("global_reward_infos");

/// Maps (staker, asset) -> reward_info
pub const STAKER_REWARD_INFOS: Map<(&str, &str), StakerRewardInfo> =
    Map::new("staker_reward_infos");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// The token accepted for staking.
    pub staking_token: AndrAddress,
    /// Any additional tokens used for rewards. Cannot include the staking token.
    pub additional_reward_tokens: Vec<AssetInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    /// The total share of the staking token in the contract.
    pub total_share: Uint128,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GlobalRewardInfo {
    /// The index of this particular reward.
    pub index: Decimal256,
    /// The reward balance to compare to when updating the index.
    pub previous_reward_balance: Uint128,
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
