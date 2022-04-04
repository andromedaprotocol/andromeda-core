use common::mission::AndrAddress;
use cosmwasm_std::{Decimal, Uint128};
use cw_asset::AssetInfo;
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const CONFIG: Item<Config> = Item::new("config");
pub const STATE: Item<State> = Item::new("state");
pub const STAKERS: Map<&str, Staker> = Map::new("stakers");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub staking_token: AndrAddress,
    pub additional_reward_tokens: Vec<AssetInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    /// The total share of the staking token in the contract.
    pub total_share: Uint128,
    /// The reward indexes for each additional reward token.
    pub additional_reward_indexes: BTreeMap<String, Decimal>,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Staker {
    /// Total staked share.
    pub share: Uint128,
    /// Reward info for each addtional reward token.
    pub reward_info: BTreeMap<String, RewardInfo>,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardInfo {
    pub index: Decimal,
    pub pending_rewards: Decimal,
}
