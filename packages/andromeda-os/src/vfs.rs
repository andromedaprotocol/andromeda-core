use common::ado_base::AndromedaQuery;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use amp::messages::AMPPkt;

#[cw_serde]
pub struct InstantiateMsg {
    /// Address of the Kernel contract on chain
    pub kernel_address: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    // Receives an AMP Packet for relaying
    // AMPReceive(AMPPkt),
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
