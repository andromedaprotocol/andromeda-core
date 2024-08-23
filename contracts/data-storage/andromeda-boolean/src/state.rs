use andromeda_data_storage::boolean::{Boolean, BooleanRestriction};
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const DATA: Item<Boolean> = Item::new("data");
pub const DATA_OWNER: Item<Addr> = Item::new("data_owner");
pub const RESTRICTION: Item<BooleanRestriction> = Item::new("restriction");
