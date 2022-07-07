use common::{
    ado_base::{AndromedaMsg, AndromedaQuery},
    OrderBy,
};
use cosmwasm_std::{Addr, Coin, Timestamp, Uint128};
use cw721::{Cw721ReceiveMsg, Expiration};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    // The cw721 contract that you want to allow NFTs from
    pub nft_contract: String,
    pub unbonding_period: u64,
    pub reward: Coin,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    ReceiveNft(Cw721ReceiveMsg),

    /// Assigns reward, and saves the unbonding time for later claim.
    Unstake {
        key: String,
    },
    Claim {
        key: String,
    },

    UpdateOwner {
        address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw721HookMsg {
    /// Stakes NFT
    Stake {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    StakedNft { key: String },
    Owner {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Bid {
    pub bidder: String,
    pub amount: Uint128,
    pub timestamp: Timestamp,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AuctionStateResponse {
    pub start_time: Expiration,
    pub end_time: Expiration,
    pub high_bidder_addr: String,
    pub high_bidder_amount: Uint128,
    pub auction_id: Uint128,
    pub coin_denom: String,
    pub whitelist: Option<Vec<Addr>>,
    pub is_cancelled: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AuctionIdsResponse {
    pub auction_ids: Vec<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct BidsResponse {
    pub bids: Vec<Bid>,
}
