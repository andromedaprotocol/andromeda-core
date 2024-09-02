use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub curve_config: CurveConfig,
    pub restriction: CurveRestriction,
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
pub enum CurveConfig {
    ExpConfig {
        curve_id: CurveId,
        base_value: u64,
        multiple_variable_value: Option<u64>,
        constant_value: Option<u64>,
    },
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    UpdateCurveConfig { curve_config: CurveConfig },
    UpdateRestriction { restriction: CurveRestriction },
    Reset {},
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetCurveConfigResponse)]
    GetCurveConfig {},
    #[returns(GetRestrictionResponse)]
    GetRestriction {},
    #[returns(GetPlotYFromXResponse)]
    GetPlotYFromX { x_value: f64 },
}

#[cw_serde]
pub struct GetCurveConfigResponse {
    pub curve_config: CurveConfig,
}

#[cw_serde]
pub struct GetPlotYFromXResponse {
    pub y_value: String,
}

#[cw_serde]
pub struct GetRestrictionResponse {
    pub restriction: CurveRestriction,
}
