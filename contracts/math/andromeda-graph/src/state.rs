use andromeda_math::graph::{CoordinateInfo, MapInfo, StoredDate};
use andromeda_math::point::PointCoordinate;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const MAP_INFO: Item<MapInfo> = Item::new("map_info");
pub const MAP_POINT_INFO: Map<&u128, (CoordinateInfo, StoredDate)> = Map::new("map_point_info");
pub const TOTAL_POINTS_NUMBER: Item<u128> = Item::new("total_points_number");
pub const USER_COORDINATE: Map<Addr, PointCoordinate> = Map::new("user_coordinate");
