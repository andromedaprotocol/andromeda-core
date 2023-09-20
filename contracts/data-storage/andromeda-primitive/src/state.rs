use andromeda_data_storage::primitive::{Primitive, PrimitiveRestriction};
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const DEFAULT_KEY: &str = "default";

pub const DATA: Map<&str, Primitive> = Map::new("data");
pub const KEY_OWNER: Map<&str, Addr> = Map::new("key_owner");
pub const RESTRICTION: Item<PrimitiveRestriction> = Item::new("restriction");
