use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::cw_serde;
use cw20::Cw20ReceiveMsg;
pub use mirror_protocol::{
    gov::{Cw20HookMsg as MirrorGovCw20HookMsg, ExecuteMsg as MirrorGovExecuteMsg},
    lock::ExecuteMsg as MirrorLockExecuteMsg,
    mint::{Cw20HookMsg as MirrorMintCw20HookMsg, ExecuteMsg as MirrorMintExecuteMsg},
    staking::{Cw20HookMsg as MirrorStakingCw20HookMsg, ExecuteMsg as MirrorStakingExecuteMsg},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cw_serde]
pub struct InstantiateMsg {
    pub primitive_contract: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    Receive(Cw20ReceiveMsg),
    MirrorMintExecuteMsg(MirrorMintExecuteMsg),
    MirrorStakingExecuteMsg(MirrorStakingExecuteMsg),
    MirrorGovExecuteMsg(MirrorGovExecuteMsg),
    MirrorLockExecuteMsg(MirrorLockExecuteMsg),
}
#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
}

#[cw_serde]
pub enum Cw20HookMsg {
    MirrorMintCw20HookMsg(MirrorMintCw20HookMsg),
    MirrorStakingCw20HookMsg(MirrorStakingCw20HookMsg),
    MirrorGovCw20HookMsg(MirrorGovCw20HookMsg),
}
