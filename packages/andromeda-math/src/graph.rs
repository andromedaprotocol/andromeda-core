use andromeda_std::amp::AndrAddr;
use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::SignedDecimal;

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
    #[attrs(restricted)]
    UpdateMap { map_info: MapInfo },
    #[attrs(restricted)]
    StoreCoordinate {
        coordinate: Coordinate,
        is_timestamp_allowed: bool,
    },
    #[attrs(restricted)]
    StoreUserCoordinate { user_location_paths: Vec<AndrAddr> },
    #[attrs(restricted)]
    DeleteUserCoordinate { user: AndrAddr },
}

#[cw_serde]
pub struct Coordinate {
    pub x_coordinate: SignedDecimal,
    pub y_coordinate: SignedDecimal,
    pub z_coordinate: Option<SignedDecimal>,
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
    GetAllPoints {
        start: Option<u128>,
        limit: Option<u32>,
    },
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
