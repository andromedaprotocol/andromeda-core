use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cosmwasm_std::Uint128;
use cw_asset::{AssetInfo, AssetInfoUnchecked};
use cw_utils::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub asset_info: AssetInfoUnchecked,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    RegisterMerkleRoot {
        /// MerkleRoot is hex-encoded merkle root.
        merkle_root: String,
        expiration: Option<Expiration>,
        total_amount: Option<Uint128>,
    },
    /// Claim does not check if contract has enough funds, owner must ensure it.
    Claim {
        stage: u8,
        amount: Uint128,
        /// Proof is hex-encoded merkle proof.
        proof: Vec<String>,
    },
    /// Burn the remaining tokens after expire time (only owner)
    Burn {
        stage: u8,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    Config {},
    MerkleRoot { stage: u8 },
    LatestStage {},
    IsClaimed { stage: u8, address: String },
    TotalClaimed { stage: u8 },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub asset_info: AssetInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MerkleRootResponse {
    pub stage: u8,
    /// MerkleRoot is hex-encoded merkle root.
    pub merkle_root: String,
    pub expiration: Expiration,
    pub total_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct LatestStageResponse {
    pub latest_stage: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct IsClaimedResponse {
    pub is_claimed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct TotalClaimedResponse {
    pub total_claimed: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct MigrateMsg {}
