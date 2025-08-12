use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

/// Maps cw20_address -> locked_amount
pub const LOCKED: Map<Addr, Uint128> = Map::new("locked");

pub const ADMINS: Item<Vec<String>> = Item::new("admins");
