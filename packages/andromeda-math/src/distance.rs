use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::SignedDecimal;

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
    #[returns(String)]
    GetManhattanDistance {
        point_1: Coordinate,
        point_2: Coordinate,
        decimal: u16,
    },
}

#[cw_serde]
pub struct Coordinate {
    pub x_coordinate: SignedDecimal,
    pub y_coordinate: SignedDecimal,
    pub z_coordinate: Option<SignedDecimal>,
}

#[cw_serde]
pub enum DistanceType {
    Straight,
    Manhattan,
}
