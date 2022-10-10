use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    // Task balancer ADO's address
    pub task_balancer: Addr,
    // Processes address
    pub process: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    /// Evaluates 2 pieces of data
    Store {
        process: Addr,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    #[returns(Addr)]
    TaskBalancer {},
    #[returns(Vec<Addr>)]
    Processes {},
}
