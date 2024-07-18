use andromeda_std::{
    andr_exec, andr_instantiate, andr_query,
};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    Split { input: String, delimiter: Delimiter },
}

#[cw_serde]
pub enum Delimiter {
    WhiteSpace,
    Other { limiter: String },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetSplitResultResponse)]
    GetSplitResult {},
}

#[cw_serde]
pub struct GetSplitResultResponse {
    pub split_result: Vec<String>,
}