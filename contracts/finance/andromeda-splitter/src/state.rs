use andromeda_finance::splitter::{Splitter, UpdatedSplitter};
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const SPLITTER: Item<Splitter> = Item::new("splitter");
pub const UPDATED_SPLITTER: Item<UpdatedSplitter> = Item::new("updated_splitter");
pub const KERNEL_ADDRESS: Item<Addr> = Item::new("kernel_address");
