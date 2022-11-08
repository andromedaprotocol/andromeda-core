use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use cw_storage_plus::Item;

#[cw_serde]
pub struct State {
    // Number of currently instantiated storage contracts
    pub contracts: Uint128,
    // Maximum number of processes that each storage contract can hold
    pub max: u64,
    // Code ID of the storage contract's that the task balancer will be instantiating
    pub storage_code_id: u64,
    // Task balancer's admin
    pub admin: String,
}

pub const STATE: Item<State> = Item::new("state");

// Storage contracts that have been instanitated by the task balancer
pub const STORAGE_CONTRACTS: Item<Vec<String>> = Item::new("storage_contracts");

// Older storage contracts with empty space
pub const UP_NEXT: Item<Vec<String>> = Item::new("older_storage_contracts_with_empty_space");
