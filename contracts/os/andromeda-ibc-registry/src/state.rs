use andromeda_std::os::ibc_registry::DenomInfo;
use cw_storage_plus::{Item, Map};

pub const REGISTRY: Map<String, DenomInfo> = Map::new("registry");

pub const SERVICE_ADDRESS: Item<String> = Item::new("service_address");
