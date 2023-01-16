use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    // Task balancer ADO's address, we can use Addr instead of String since the task balancer instantiates the contract
    pub task_balancer: Addr,
    // Processes address, we can use Addr instead of String since the task balancer validates the process's address
    pub process: Addr,
    // Max number of processes
    pub max_processes: u64,
    pub kernel_address: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    // Stores process
    Store { process: String },
    // Removes process
    Remove { process: String },
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
    #[returns(bool)]
    FreeSpace {},
    #[returns(bool)]
    HasProcess { process: Addr },
}
