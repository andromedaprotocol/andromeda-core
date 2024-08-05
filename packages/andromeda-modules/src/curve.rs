use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub curve_type: CurveType,
    pub restriction: CurveRestriction,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    UpdateCurveType {
        curve_type: CurveType,
    },
    UpdateRestriction {
        restriction: CurveRestriction,
    },
    ConfigureExponential {
        curve_id: CurveId,
        base_value: u64,
        multiple_variable_value: Option<u64>,
        constant_value: Option<u64>,
    },
    Reset {},
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetCurveTypeResponse)]
    GetCurveType {},
    #[returns(GetConfigurationExpResponse)]
    GetConfigurationExp {},
    #[returns(GetRestrictionResponse)]
    GetRestriction {},
    #[returns(GetPlotYFromXResponse)]
    GetPlotYFromX { x_value: f64 },
}

#[cw_serde]
pub enum CurveType {
    Exponential,
}

#[cw_serde]
pub enum CurveRestriction {
    Private,
    Public,
}

#[cw_serde]
pub enum CurveId {
    Growth,
    Decay,
}

#[cw_serde]
pub struct GetCurveTypeResponse {
    pub curve_type: CurveType,
}

#[cw_serde]
pub struct GetConfigurationExpResponse {
    pub curve_id: CurveId,
    pub base_value: u64,
    pub multiple_variable_value: u64,
    pub constant_value: u64,
}

#[cw_serde]
pub struct GetPlotYFromXResponse {
    pub y_value: String,
}

#[cw_serde]
pub struct GetRestrictionResponse {
    pub restriction: CurveRestriction,
}
