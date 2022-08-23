use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cw20::Cw20ReceiveMsg;
pub use mirror_protocol::{
    gov::{Cw20HookMsg as MirrorGovCw20HookMsg, ExecuteMsg as MirrorGovExecuteMsg},
    lock::ExecuteMsg as MirrorLockExecuteMsg,
    mint::{Cw20HookMsg as MirrorMintCw20HookMsg, ExecuteMsg as MirrorMintExecuteMsg},
    staking::{Cw20HookMsg as MirrorStakingCw20HookMsg, ExecuteMsg as MirrorStakingExecuteMsg},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub primitive_contract: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    Receive(Cw20ReceiveMsg),
    MirrorMintExecuteMsg(MirrorMintExecuteMsg),
    MirrorStakingExecuteMsg(MirrorStakingExecuteMsg),
    MirrorGovExecuteMsg(MirrorGovExecuteMsg),
    MirrorLockExecuteMsg(MirrorLockExecuteMsg),
}
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    MirrorMintCw20HookMsg(MirrorMintCw20HookMsg),
    MirrorStakingCw20HookMsg(MirrorStakingCw20HookMsg),
    MirrorGovCw20HookMsg(MirrorGovCw20HookMsg),
}
