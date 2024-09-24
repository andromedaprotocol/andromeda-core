use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(String)]
    GetDistanceBetween2Points {
        point_1: Coordinate,
        point_2: Coordinate,
        decimal: u16,
    },
}

#[cw_serde]
pub struct Coordinate {
    pub x_coordinate: f64,
    pub y_coordinate: f64,
    pub z_coordinate: Option<f64>,
}
