use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub expressions: Vec<String>,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    UpdateExpressions { expressions: Vec<String> },
}

#[cw_serde]
pub struct MigrateMsg {}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ShuntingResponse)]
    Evaluate { params: Vec<EvaluateParam> },
}

#[cw_serde]
pub enum EvaluateParam {
    Value(String),
    Reference(EvaluateRefParam),
}

#[cw_serde]
pub struct EvaluateRefParam {
    pub contract: Addr,
    pub msg: String,
    pub accessor: String,
}

#[cw_serde]
pub struct ShuntingResponse {
    pub result: String,
}
