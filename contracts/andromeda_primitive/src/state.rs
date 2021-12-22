use andromeda_protocol::primitive::Primitive;
use cw_storage_plus::Map;

pub const DEFAULT_KEY: &str = "default";

pub const DATA: Map<&str, Primitive> = Map::new("data");
