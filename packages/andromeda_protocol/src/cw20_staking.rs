use common::{
    ado_base::{AndromedaMsg, AndromedaQuery},
    mission::AndrAddress,
};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use cw_asset::AssetInfoUnchecked;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMsg {
    /// The cw20 token that can be staked.
    pub staking_token: AndrAddress,
    /// Any rewards in addition to the staking token. This list cannot include the staking token.
    pub additional_rewards: Option<Vec<AssetInfoUnchecked>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    AndrReceive(AndromedaMsg),
    /// Add `asset_info` as another reward token. Owner only.
    AddRewardToken {
        asset_info: AssetInfoUnchecked,
    },
    /// Unstakes the specified amount of assets, or all if not specified. The user's pending
    /// rewards and indexes are updated for each additional reward token.
    UnstakeTokens {
        amount: Option<Uint128>,
    },
    /// Claims any outstanding rewards from the addtional reward tokens.
    ClaimRewards {},
    /// Updates the global reward index for the specified assets or all of the specified ones if
    /// None. Funds may be sent along with this.
    UpdateGlobalIndexes {
        asset_infos: Option<Vec<AssetInfoUnchecked>>,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub enum Cw20HookMsg {
    /// Stake the sent tokens. Address must match the `staking_token` given on instantiation. The user's pending
    /// rewards and indexes are updated for each additional reward token.
    StakeTokens {},
    /// Updates the global reward index on deposit of a valid cw20 token.
    UpdateGlobalIndex {},
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    /// Gets the config of the contract.
    Config {},
    /// Gets the state of the contract.
    State {},
    /// Returns a `StakerResponse` for the given staker. The pending rewards are updated to the
    /// present index.
    Staker {
        address: String,
    },
    /// Returns a `Vec<StakerResponse>` for range of stakers. The pending rewards are updated to the
    /// present index for each staker.
    Stakers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct StakerResponse {
    pub address: String,
    pub share: Uint128,
    pub pending_rewards: Vec<(String, Uint128)>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub enum MigrateMsg {}
