use common::{
    ado_base::{AndromedaMsg, AndromedaQuery},
    app::AndrAddress,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    // Target ADO's address
    pub target_address: AndrAddress,
    // Condition ADO's address
    pub condition_address: AndrAddress,
    // Desired increment
    pub increment: Increment,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    /// Evaluates 2 pieces of data
    Execute {},
    UpdateConditionAddress {
        condition_address: AndrAddress,
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
    TargetADO {},
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum Increment {
    One,
    Two,
}
