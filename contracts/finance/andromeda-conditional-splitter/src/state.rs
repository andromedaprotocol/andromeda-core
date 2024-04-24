use andromeda_finance::conditional_splitter::ConditionalSplitter;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

pub const CONDITIONAL_SPLITTER: Item<ConditionalSplitter> = Item::new("conditional_splitter");
pub const KERNEL_ADDRESS: Item<Addr> = Item::new("kernel_address");
