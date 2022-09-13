use common::{
    ado_base::{AndromedaMsg, AndromedaQuery},
    app::AndrAddress,
};
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    // Execute ADO's address
    pub condition_address: AndrAddress,

    // Query ADO's address
    pub query_address: AndrAddress,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    /// Evaluates 2 pieces of data
    Evaluate {
        user_value: Uint128,
        operation: Operators,
    },
    ChangeConditionAddress {
        address: AndrAddress,
    },
    ChangeQueryAddress {
        address: AndrAddress,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    ConditionADO {},
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
