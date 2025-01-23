use andromeda_std::{
    andr_exec, andr_instantiate, andr_query,
    common::{denom::Asset, expiration::Expiry, MillisecondsExpiration},
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub asset_info: Asset,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    #[attrs(restricted, nonpayable)]
    RegisterMerkleRoot {
        /// MerkleRoot is hex-encoded merkle root.
        merkle_root: String,
        expiration: Option<Expiry>,
        total_amount: Option<Uint128>,
    },
    /// Claim does not check if contract has enough funds, owner must ensure it.
    #[attrs(nonpayable)]
    Claim {
        stage: u8,
        amount: Uint128,
        /// Proof is hex-encoded merkle proof.
        proof: Vec<String>,
    },
    /// Burn the remaining tokens after expire time (only owner)
    #[attrs(restricted)]
    Burn { stage: u8 },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
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
    pub asset_info: Asset,
}

#[cw_serde]
pub struct MerkleRootResponse {
    pub stage: u8,
    /// MerkleRoot is hex-encoded merkle root.
    pub merkle_root: String,
    pub expiration: Option<MillisecondsExpiration>,
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
