use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

pub const MINT_RECIPIENT_AMOUNT: Item<(String, Uint128)> = Item::new("mint_recipient_amount");
/// Temporary storage for CW20 address during denom creation
pub const CW20_FOR_DENOM: Item<Addr> = Item::new("cw20_for_denom");

/// Maps cw20_address -> locked_amount
pub const LOCKED: Map<Addr, Uint128> = Map::new("locked");

/// Maps cw20_address -> factory_denom
pub const FACTORY_DENOMS: Map<Addr, String> = Map::new("factory_denoms");

pub const OSMOSIS_MSG_CREATE_DENOM_ID: u64 = 1;
pub const OSMOSIS_MSG_BURN_ID: u64 = 2;
