use cosmwasm_std::Addr;
use cw_storage_plus::Map;

pub const IBC_BRIDGE: &str = "ibc-bridge";

pub const KERNEL_ADDRESSES: Map<&str, Addr> = Map::new("kernel_addresses");
