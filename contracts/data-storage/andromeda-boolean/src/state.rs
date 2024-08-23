use andromeda_data_storage::boolean::BooleanRestriction;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const DATA: Item<bool> = Item::new("data");
pub const DATA_OWNER: Item<Addr> = Item::new("data_owner");
pub const RESTRICTION: Item<BooleanRestriction> = Item::new("restriction");
