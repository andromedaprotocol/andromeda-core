use common::{
    ado_base::{modules::Module, AndromedaMsg, AndromedaQuery},
    mission::AndrAddress,
};
use cw20::Cw20ReceiveMsg;
use cw_asset::{AssetInfoUnchecked, AssetListUnchecked};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMsg {
    /// The cw20 token that can be staked.
    pub staking_token: AndrAddress,
    /// Any rewards in addition to the base token.
    pub additional_rewards: Option<Vec<AssetInfoUnchecked>>,
    /// Optional modules.
    pub modules: Option<Vec<Module>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    AndrReceive(AndromedaMsg),
    /// Add `asset_info` as another reward token. Owner only.
    AddRewardToken {
        asset_info: AssetInfoUnchecked,
    },
    /// Withdraw specified assets, or all of them if not specified.
    WithdrawTokens {
        assets: Option<AssetListUnchecked>,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub enum Cw20HookMsg {
    /// Stake the sent tokens. Address must match the `staking_token` given in instantiation.
    StakeTokens {},
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub enum MigrateMsg {}
