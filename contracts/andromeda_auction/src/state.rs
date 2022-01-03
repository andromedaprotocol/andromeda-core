use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub token_addr: String,
    pub stable_denom: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenAuctionState{
    pub start_time: u64,
    pub end_time: u64,
    pub high_bidder_addr: Addr,
    pub high_bidder_amount: Uint128
}
pub const CONFIG: Item<Config> = Item::new("config");
pub const FUNDS_BY_BIDDER:Map<String, Vec<(String, Uint128)>> = Map::new("funds_by_bidder");
pub const TOKEN_AUCTION_STATE: Map<String, TokenAuctionState> = Map::new("auction_token_state");
// pub const OWNER_HAS_WITHDRAWN: Map<String, bool> = Map::new("owner_has_withdrawn");


