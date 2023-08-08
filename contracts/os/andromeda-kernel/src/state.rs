use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const IBC_BRIDGE: &str = "ibc-bridge";

pub const KERNEL_ADDRESSES: Map<&str, Addr> = Map::new("kernel_addresses");

//Temporary storage for creating a new ADO to assign a new owner
pub const ADO_OWNER: Item<Addr> = Item::new("ado_owner");
