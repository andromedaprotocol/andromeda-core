use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct State {
    // Total number of contracts branched from the router
    pub contracts: Uint128,
    // MAX of each size of MAP in each branch contract
    pub max: u64,
    pub storage_code_id: u64,
    pub admin: String,
}

pub const STATE: Item<State> = Item::new("state");
// Specify KV Pair
pub const CONTRACTS: Map<String, String> = Map::new("contracts");
