use cosmwasm_std::{String, Uint128};
use cw_storage_plus::Map;

pub const BALANCES: Map<&String, Uint128> = Map::new("balance");
