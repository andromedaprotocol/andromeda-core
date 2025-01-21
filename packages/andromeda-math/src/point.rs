use andromeda_std::{amp::AndrAddr, andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::SignedDecimal;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub restriction: PointRestriction,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    SetPoint { point: PointCoordinate },
    DeletePoint {},
    UpdateRestriction { restriction: PointRestriction },
}

#[cw_serde]
pub enum PointRestriction {
    Private,
    Public,
    Restricted,
}

#[cw_serde]
pub struct PointCoordinate {
    pub x_coordinate: SignedDecimal,
    pub y_coordinate: SignedDecimal,
    pub z_coordinate: Option<SignedDecimal>,
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(PointCoordinate)]
    GetPoint {},
    #[returns(GetDataOwnerResponse)]
    GetDataOwner {},
}

#[cw_serde]
pub struct GetDataOwnerResponse {
    pub owner: AndrAddr,
}
