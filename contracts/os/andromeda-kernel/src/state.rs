use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const ADO_DB_KEY: &str = "adodb";
pub const VFS_KEY: &str = "vfs";

pub const KERNEL_ADDRESSES: Map<&str, Addr> = Map::new("kernel_addresses");
pub const IBC_BRIDGE: Item<Addr> = Item::new("ibc_bridge");
