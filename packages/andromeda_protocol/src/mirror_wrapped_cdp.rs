pub use mirror_protocol::{
    gov::{ExecuteMsg as MirrorGovExecuteMsg, QueryMsg as MirrorGovQueryMsg},
    mint::{ExecuteMsg as MirrorMintExecuteMsg, QueryMsg as MirrorMintQueryMsg},
    staking::{ExecuteMsg as MirrorStakingExecuteMsg, QueryMsg as MirrorStakingQueryMsg},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub mirror_mint_contract: String,
    pub mirror_staking_contract: String,
    pub mirror_gov_contract: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    MirrorMintExecuteMsg(MirrorMintExecuteMsg),
    MirrorStakingExecuteMsg(MirrorStakingExecuteMsg),
    MirrorGovExecuteMsg(MirrorGovExecuteMsg),
    UpdateOwner {
        address: String,
    },
    UpdateConfig {
        mirror_mint_contract: Option<String>,
        mirror_staking_contract: Option<String>,
        mirror_gov_contract: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    MirrorMintQueryMsg(MirrorMintQueryMsg),
    MirrorStakingQueryMsg(MirrorStakingQueryMsg),
    MirrorGovQueryMsg(MirrorGovQueryMsg),
    ContractOwner {},
    Config {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub mirror_mint_contract: String,
    pub mirror_staking_contract: String,
    pub mirror_gov_contract: String,
}
