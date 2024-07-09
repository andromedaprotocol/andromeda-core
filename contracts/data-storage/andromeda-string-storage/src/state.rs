use andromeda_data_storage::string_storage::{StringStorage, StringStorageRestriction};
use cw_storage_plus::Item;
use cosmwasm_std::Addr;

pub const DATA: Item<StringStorage> = Item::new("data");
pub const DATA_OWNER: Item<Addr> = Item::new("data_owner");
pub const RESTRICTION: Item<StringStorageRestriction> = Item::new("restriction");