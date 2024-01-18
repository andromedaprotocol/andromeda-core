use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Map;

/// Contains all balances for an address
pub const BALANCES: Map<(Addr, String), Uint128> = Map::new("balances");
