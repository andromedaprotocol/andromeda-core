use cw20::Cw20ReceiveMsg;
pub use mirror_protocol::{
    gov::{Cw20HookMsg as MirrorGovCw20HookMsg, ExecuteMsg as MirrorGovExecuteMsg},
    lock::ExecuteMsg as MirrorLockExecuteMsg,
    mint::{Cw20HookMsg as MirrorMintCw20HookMsg, ExecuteMsg as MirrorMintExecuteMsg},
    staking::{Cw20HookMsg as MirrorStakingCw20HookMsg, ExecuteMsg as MirrorStakingExecuteMsg},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub mirror_mint_contract: String,
    pub mirror_staking_contract: String,
    pub mirror_gov_contract: String,
    pub mirror_lock_contract: String,
    pub operators: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    MirrorMintExecuteMsg(MirrorMintExecuteMsg),
    MirrorStakingExecuteMsg(MirrorStakingExecuteMsg),
    MirrorGovExecuteMsg(MirrorGovExecuteMsg),
    MirrorLockExecuteMsg(MirrorLockExecuteMsg),
    UpdateOwner {
        address: String,
    },
    UpdateConfig {
        mirror_mint_contract: Option<String>,
        mirror_staking_contract: Option<String>,
        mirror_gov_contract: Option<String>,
        mirror_lock_contract: Option<String>,
    },
    UpdateOperators {
        operators: Vec<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    ContractOwner {},
    Config {},
    IsOperator { address: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    MirrorMintCw20HookMsg(MirrorMintCw20HookMsg),
    MirrorStakingCw20HookMsg(MirrorStakingCw20HookMsg),
    MirrorGovCw20HookMsg(MirrorGovCw20HookMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub mirror_mint_contract: String,
    pub mirror_staking_contract: String,
    pub mirror_gov_contract: String,
    pub mirror_lock_contract: String,
}
