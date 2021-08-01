use cosmwasm_std::{HumanAddr, Uint128};
use cw_storage_plus::Map;

pub const BALANCES: Map<&HumanAddr, Uint128> = Map::new("balance");
