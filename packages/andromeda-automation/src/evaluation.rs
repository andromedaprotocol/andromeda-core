use common::{
    ado_base::{AndromedaMsg, AndromedaQuery},
    app::AndrAddress,
};
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    // Condition ADO's address
    pub condition_address: AndrAddress,

    // Query ADO's address
    pub query_address: AndrAddress,

    // Task balancer ADO's address
    pub task_balancer: AndrAddress,

    // The value we want to compare with the oracle's
    pub user_value: Uint128,

    // Sets the way we want to compare the Oracle's value to the other's. Either greater, less ...
    pub operation: Operators,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    ChangeConditionAddress { address: AndrAddress },
    ChangeQueryAddress { address: AndrAddress },
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    ConditionADO {},
    Evaluation {},
    QueryADO {},
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq, Eq)]
pub enum Operators {
    Greater,
    GreaterEqual,
    Equal,
    LessEqual,
    Less,
}
