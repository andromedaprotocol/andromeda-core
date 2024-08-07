use andromeda_data_storage::graph::{CoordinateInfo, MapInfo, StoredDate};
use cw_storage_plus::{Item, Map};

pub const MAP_INFO: Item<MapInfo> = Item::new("map_info");
pub const MAP_POINT_INFO: Map<&u128, (CoordinateInfo, StoredDate)> = Map::new("map_point_info");
pub const POINT: Item<u128> = Item::new("point");
