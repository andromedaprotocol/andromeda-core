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
    /// Any rewards in addition to the base token.
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
    /// Unstakes the specified amount of assets, or all if not specified.
    UnstakeTokens {
        amount: Option<Uint128>,
    },
    ClaimRewards {},
    /// Updates the global reward index for the specified assets or all of the specified ones if
    /// None. Funds may be sent along with this.
    UpdateGlobalIndexes {
        asset_infos: Option<Vec<AssetInfoUnchecked>>,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub enum Cw20HookMsg {
    /// Stake the sent tokens. Address must match the `staking_token` given in instantiation. Upon
    /// deposit the user's pending reward and user index are updated.
    StakeTokens {},
    /// Updates the global reward index on deposit of whitelisted cw20 tokens.
    UpdateGlobalRewardIndexes {},
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub enum MigrateMsg {}
