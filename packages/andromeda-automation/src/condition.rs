use common::{
    ado_base::{AndromedaMsg, AndromedaQuery},
    app::AndrAddress,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub logic_gate: LogicGate,
    pub whitelist: Vec<String>,
    pub execute_ado: AndrAddress,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    /// Executes based off the evaluation ADO's bool
    Interpret {},
    StoreResult {
        result: bool,
    },
    UpdateExecuteADO {
        address: AndrAddress,
    },
    UpdateWhitelist {
        addresses: Vec<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    LogicGate {},
    Whitelist {},
    Results {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum LogicGate {
    AND,
    OR,
    XOR,
    NOT,
    NAND,
    NOR,
    XNOR,
}
