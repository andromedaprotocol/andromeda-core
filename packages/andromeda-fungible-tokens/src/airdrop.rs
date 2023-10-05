use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw_asset::{AssetInfo, AssetInfoUnchecked};
use cw_utils::Expiration;

#[cw_serde]
pub struct InstantiateMsg {
    pub asset_info: AssetInfoUnchecked,
}

#[cw_serde]
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

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    #[returns(ConfigResponse)]
    Config {},
    #[returns(MerkleRootResponse)]
    MerkleRoot { stage: u8 },
    #[returns(LatestStageResponse)]
    LatestStage {},
    #[returns(IsClaimedResponse)]
    IsClaimed { stage: u8, address: String },
    #[returns(TotalClaimedResponse)]
    TotalClaimed { stage: u8 },
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub asset_info: AssetInfo,
}

#[cw_serde]
pub struct MerkleRootResponse {
    pub stage: u8,
    /// MerkleRoot is hex-encoded merkle root.
    pub merkle_root: String,
    pub expiration: Expiration,
    pub total_amount: Uint128,
}

#[cw_serde]
pub struct LatestStageResponse {
    pub latest_stage: u8,
}

#[cw_serde]
pub struct IsClaimedResponse {
    pub is_claimed: bool,
}

#[cw_serde]
pub struct TotalClaimedResponse {
    pub total_claimed: Uint128,
}

#[cw_serde]
pub struct MigrateMsg {}
