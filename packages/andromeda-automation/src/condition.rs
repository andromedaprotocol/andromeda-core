use common::{
    ado_base::{AndromedaMsg, AndromedaQuery},
    app::AndrAddress,
};
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::evaluation::Operators;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub logic_gate: LogicGate,
    pub whitelist: Vec<EvalDetails>,
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
    GetResult {
        user_value: Uint128,
        operation: Operators,
    },
    UpdateExecuteADO {
        address: AndrAddress,
    },
    UpdateWhitelist {
        addresses: Vec<EvalDetails>,
    },
    UpdateLogicGate {
        logic_gate: LogicGate,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct EvalDetails {
    pub contract_addr: String,
    pub user_value: Uint128,
    pub operation: Operators,
}
