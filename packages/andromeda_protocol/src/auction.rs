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
    PlaceBid {
        token_id: String,
    },
    Withdraw {
        token_id: String,
    },
    StartAuction {
        token_id: String,
        start_time: u64,
        end_time: u64,
        stable_denom: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AuctionState { token_id: String },
}

// #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
// pub struct HighestBidResponse{
//     pub address: String,
//     pub bid: Uint128,
//
// }
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AuctionStateResponse {
    pub start_time: u64,
    pub end_time: u64,
    pub high_bidder_addr: String,
    pub high_bidder_amount: Uint128,
}
