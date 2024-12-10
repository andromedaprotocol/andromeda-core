use andromeda_std::os::ibc_registry::DenomInfo;
use cw_storage_plus::Map;

pub const REGISTRY: Map<String, DenomInfo> = Map::new("registry");
