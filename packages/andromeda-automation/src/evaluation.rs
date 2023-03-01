use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {
    // Condition ADO's address
    pub condition_address: String,

    // Oracle ADO's address
    pub oracle_address: String,

    // Task balancer ADO's address
    pub task_balancer: String,

    // The value we want to compare with the oracle's, if absent, we assume that the oracle is returning a bool
    pub user_value: Option<Uint128>,

    // Sets the way we want to compare the Oracle's value to the other's. Either greater, less ...
    pub operation: Operators,

    pub kernel_address: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    ChangeConditionAddress { address: String },
    ChangeQueryAddress { address: String },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),

    #[returns(String)]
    ConditionADO {},

    #[returns(bool)]
    Evaluation {},

    #[returns(String)]
    OracleADO {},
}

#[cw_serde]
pub enum Operators {
    Greater,
    GreaterEqual,
    Equal,
    LessEqual,
    Less,
}
