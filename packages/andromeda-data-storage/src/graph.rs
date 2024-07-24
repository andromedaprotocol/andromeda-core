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
    pub x_length: u64,
    pub y_length: u64,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    UpdateMap {  
        map_info: MapInfo 
    },
    StoreCoordinate {
        coordinate: Coordinate,
    },
}

#[cw_serde]
pub struct Coordinate {
    pub x_coordinate: f64,
    pub y_coordinate: f64,
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetMapInfoResponse)]
    GetMapInfo {},
    #[returns(GetMaxPointResponse)]
    GetMaxPoint {},
    #[returns(GetAllPointsResponse)]
    GetAllPoints {},
}

#[cw_serde]
pub struct GetMapInfoResponse {
    pub map_info: MapInfo,
}

#[cw_serde]
pub struct GetMaxPointResponse {
    pub max_point: u128,
}

#[cw_serde]
pub struct GetAllPointsResponse {
    pub points: Vec<CoordinateResponse>,
}

#[cw_serde]
pub struct CoordinateResponse {
    pub x: String,
    pub y: String,
}
