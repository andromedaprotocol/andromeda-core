use common::{
    ado_base::{AndromedaMsg, AndromedaQuery},
    app::AndrAddress,
};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
pub struct InstantiateMsg {
    pub logic_gate: LogicGate,
    pub whitelist: Vec<String>,
    pub execute_ado: AndrAddress,
}

#[cw_serde]
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
    UpdateLogicGate {
        logic_gate: LogicGate,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    #[returns(LogicGate)]
    LogicGate {},
    #[returns(Vec<String>)]
    Whitelist {},
    #[returns(Vec<bool>)]
    Results {},
}

#[cw_serde]
pub enum LogicGate {
    AND,
    OR,
    XOR,
    NOT,
    NAND,
    NOR,
    XNOR,
}
