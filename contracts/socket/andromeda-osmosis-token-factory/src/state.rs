use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Map;

/// Maps cw20_address -> locked_amount
pub const LOCKED: Map<Addr, Uint128> = Map::new("locked");

/// Maps cw20_address -> factory_denom
pub const FACTORY_DENOMS: Map<Addr, String> = Map::new("factory_denoms");

pub const DENOMS_TO_OWNER: Map<String, Addr> = Map::new("denoms_to_owner");

pub const OSMOSIS_MSG_BURN_ID: u64 = 1;
