use common::{
    ado_base::{AndromedaMsg, AndromedaQuery},
    app::AndrAddress,
};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
pub struct InstantiateMsg {
    pub logic_gate: LogicGate,
    pub eval_ados: Vec<AndrAddress>,
    pub execute_ado: AndrAddress,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),

    /// Executes based off the evaluation ADO's bool
    Interpret {
        results: Vec<bool>,
    },

    GetResults {},
    UpdateExecuteADO {
        address: AndrAddress,
    },
    UpdateWhitelist {
        addresses: Vec<AndrAddress>,
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
    #[returns(Vec<AndrAddress>)]
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
