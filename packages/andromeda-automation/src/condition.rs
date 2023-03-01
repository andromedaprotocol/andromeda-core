use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
pub struct InstantiateMsg {
    pub logic_gate: LogicGate,
    pub eval_ados: Vec<String>,
    pub execute_ado: String,
    pub kernel_address: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    // Gets the results from the Eval ADOs and then interprets them based off the selected logic gate
    GetResults {},
    UpdateExecuteADO { address: String },
    UpdateEvalAdos { addresses: Vec<String> },
    UpdateLogicGate { logic_gate: LogicGate },
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
    EvalAdos {},
}

#[cw_serde]
pub enum LogicGate {
    And,
    Or,
    Xor,
    Not,
    Nand,
    Nor,
    Xnor,
}
