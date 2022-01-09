use crate::common::OrderBy;
use cosmwasm_std::{Timestamp, Uint128};
use cw721::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub token_addr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Places a bid on the current auction for the given token_id. The previous largest bid gets
    /// automatically sent back to the bidder when they are outbid.
    PlaceBid {
        token_id: String,
    },
    /// Transfers the given token to the auction winner's address once the auction is over.
    Claim {
        token_id: String,
    },
    /// Starts a new auction with the given parameters. The auction info can be modified before it
    /// has started but is immutable after that.
    StartAuction {
        token_id: String,
        start_time: Expiration,
        end_time: Expiration,
        coin_denom: String,
    },
    UpdateOwner {
        address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Gets the latest auction state for the given token. This will either be the current auction
    /// if there is one in progress or the last completed one.
    LatestAuctionState {
        token_id: String,
    },
    /// Gets the auction state for the given auction id.
    AuctionState {
        auction_id: Uint128,
    },
    /// Gets the config.
    Config {},
    /// Gets the bids for the given auction id. Start_after starts indexing at 0.
    Bids {
        auction_id: Uint128,
        start_after: Option<u64>,
        limit: Option<u64>,
        order_by: Option<OrderBy>,
    },
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
    pub claimed: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ConfigResponse {
    pub token_addr: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct BidsResponse {
    pub bids: Vec<Bid>,
}
