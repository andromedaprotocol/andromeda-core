use common::ado_base::AndromedaQuery;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use amp::messages::AMPPkt;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Receives an AMP Packet for relaying
    Receive(AMPPkt),
    /// Upserts a key address to the kernel, restricted to the owner of the kernel
    UpsertKeyAddress { key: String, value: String },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    #[returns(Addr)]
    KeyAddress { key: String },
    #[returns(bool)]
    VerifyAddress { address: String },
}
