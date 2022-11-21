use common::{
    ado_base::{AndromedaMsg, AndromedaQuery},
    app::AndrAddress,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;

#[cw_serde]
pub struct InstantiateMsg {
    // The contract we'll send the ExecuteMsg to
    pub target_address: AndrAddress,
    // Condition ADO's address
    pub condition_address: AndrAddress,
    // Desired increment
    pub increment: Increment,
    // Task balancer's address
    pub task_balancer: String,
    // Target ADO's Execute Msg
    pub target_message: Binary,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    /// Evaluates 2 pieces of data
    Execute {},
    UpdateConditionAddress {
        condition_address: AndrAddress,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    #[returns(String)]
    ConditionADO {},
    #[returns(String)]
    TargetADO {},
}
#[cw_serde]
pub enum Increment {
    One,
    Two,
}
