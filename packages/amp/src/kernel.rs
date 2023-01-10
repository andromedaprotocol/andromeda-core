use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::{cw_serde, QueryResponses};

use crate::messages::AMPPkt;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Receives an AMP Packet for relaying
    Receive(AMPPkt),
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
}
