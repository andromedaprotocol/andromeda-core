use andromeda_fungible_tokens::exchange::{Redeem, Sale};
use andromeda_std::amp::AndrAddr;
use cw_storage_plus::{Item, Map};

pub const TOKEN_ADDRESS: Item<AndrAddr> = Item::new("token_address");
pub const SALE: Map<&str, Sale> = Map::new("sale");
pub const REDEEM: Map<&str, Redeem> = Map::new("redeem");
