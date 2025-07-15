use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Map, Item};


/// Maps (owner, cw20_address) -> locked_amount
pub const LOCKED: Map<(Addr, Addr), Uint128> = Map::new("locked");

/// Maps cw20_address -> factory_denom
pub const FACTORY_DENOMS: Map<Addr, String> = Map::new("factory_denoms");

/// Stores pending mint info (user_addr, amount, cw20_addr) during denom creation
pub const PENDING_MINT: Item<(Addr, Uint128, Addr, String)> = Item::new("pending_mint");

/// Reply IDs for async operations
pub const CREATE_DENOM_REPLY_ID: u64 = 1;
pub const MINT_REPLY_ID: u64 = 2; 