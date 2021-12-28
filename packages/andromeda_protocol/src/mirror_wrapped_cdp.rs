use cw20::Cw20ReceiveMsg;
pub use mirror_protocol::{
    collateral_oracle::QueryMsg as MirrorCollateralOracleQueryMsg,
    gov::{
        Cw20HookMsg as MirrorGovCw20HookMsg, ExecuteMsg as MirrorGovExecuteMsg,
        QueryMsg as MirrorGovQueryMsg,
    },
    lock::{ExecuteMsg as MirrorLockExecuteMsg, QueryMsg as MirrorLockQueryMsg},
    mint::{
        Cw20HookMsg as MirrorMintCw20HookMsg, ExecuteMsg as MirrorMintExecuteMsg,
        QueryMsg as MirrorMintQueryMsg,
    },
    oracle::QueryMsg as MirrorOracleQueryMsg,
    staking::{
        Cw20HookMsg as MirrorStakingCw20HookMsg, ExecuteMsg as MirrorStakingExecuteMsg,
        QueryMsg as MirrorStakingQueryMsg,
    },
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub mirror_mint_contract: String,
    pub mirror_staking_contract: String,
    pub mirror_gov_contract: String,
    pub mirror_lock_contract: String,
    pub mirror_oracle_contract: String,
    pub mirror_collateral_oracle_contract: String,
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
        mirror_oracle_contract: Option<String>,
        mirror_collateral_oracle_contract: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    MirrorMintQueryMsg(MirrorMintQueryMsg),
    MirrorStakingQueryMsg(MirrorStakingQueryMsg),
    MirrorGovQueryMsg(MirrorGovQueryMsg),
    MirrorLockQueryMsg(MirrorLockQueryMsg),
    MirrorOracleQueryMsg(MirrorOracleQueryMsg),
    MirrorCollateralOracleQueryMsg(MirrorCollateralOracleQueryMsg),
    ContractOwner {},
    Config {},
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
    pub mirror_oracle_contract: String,
    pub mirror_collateral_oracle_contract: String,
}
