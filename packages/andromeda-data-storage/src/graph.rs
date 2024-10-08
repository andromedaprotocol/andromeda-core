use andromeda_std::amp::AndrAddr;
use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub map_info: MapInfo,
}

#[cw_serde]
pub struct MapInfo {
    pub map_size: MapSize,
    pub allow_negative: bool,
    pub map_decimal: u16,
}

#[cw_serde]
pub struct MapSize {
    pub x_width: u64,
    pub y_width: u64,
    pub z_width: Option<u64>,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    UpdateMap {
        map_info: MapInfo,
    },
    StoreCoordinate {
        coordinate: Coordinate,
        is_timestamp_allowed: bool,
    },
    StoreUserCoordinate {
        user_location_paths: Vec<AndrAddr>,
    },
    DeleteUserCoordinate {
        user: AndrAddr,
    },
}

#[cw_serde]
pub struct Coordinate {
    pub x_coordinate: f64,
    pub y_coordinate: f64,
    pub z_coordinate: Option<f64>,
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetMapInfoResponse)]
    GetMapInfo {},
    #[returns(GetMaxPointNumberResponse)]
    GetMaxPointNumber {},
    #[returns(GetAllPointsResponse)]
    GetAllPoints {},
    #[returns(CoordinateInfo)]
    GetUserCoordinate { user: AndrAddr },
}

#[cw_serde]
pub struct GetMapInfoResponse {
    pub map_info: MapInfo,
}

#[cw_serde]
pub struct GetMaxPointNumberResponse {
    pub max_point_number: u128,
}

#[cw_serde]
pub struct GetAllPointsResponse {
    pub points: Vec<(CoordinateInfo, StoredDate)>,
}

#[cw_serde]
pub struct CoordinateInfo {
    pub x: String,
    pub y: String,
    pub z: Option<String>,
}

#[cw_serde]
pub struct StoredDate {
    pub timestamp: Option<u64>,
}
