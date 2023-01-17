use andromeda_finance::splitter::Splitter;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const SPLITTER: Item<Splitter> = Item::new("splitter");
pub const KERNEL_ADDRESS: Item<Addr> = Item::new("kernel_address");
