use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const DEFAULT_VALIDATOR: Item<Addr> = Item::new("default_validator");
