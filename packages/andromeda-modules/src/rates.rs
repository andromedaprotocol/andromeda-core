use andromeda_std::{ado_base::rates::LocalRate, andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub action: String,
    pub rate: LocalRate,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    #[attrs(restricted, nonpayable)]
    SetRate { action: String, rate: LocalRate },
    #[attrs(restricted, nonpayable)]
    RemoveRate { action: String },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(RateResponse)]
    Rate { action: String },
}

#[cw_serde]
pub struct RateResponse {
    pub rate: LocalRate,
}
