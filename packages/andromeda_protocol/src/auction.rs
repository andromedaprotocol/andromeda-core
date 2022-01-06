use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub token_addr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Places a bid on the current auction for the given token_id. If a bid already exists and the
    /// bidder is not the highest bidder, the amount sent will be added to the previous bid.
    PlaceBid { token_id: String },
    /// Transfers the given token to the auction winner's address once the auction is over.
    Claim { token_id: String },
    /// Withdraws the sender's bid as long as they are not the highest bid.
    Withdraw { auction_id: Uint128 },
    /// Starts a new auction with the given parameters. The auction info can be modified before it
    /// has started but is immutable after that.
    StartAuction {
        token_id: String,
        start_time: u64,
        end_time: u64,
        coin_denom: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    LatestAuctionState { token_id: String },
    AuctionState { auction_id: Uint128 },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AuctionStateResponse {
    pub start_time: u64,
    pub end_time: u64,
    pub high_bidder_addr: String,
    pub high_bidder_amount: Uint128,
    pub auction_id: Uint128,
    pub coin_denom: String,
    pub claimed: bool,
}
