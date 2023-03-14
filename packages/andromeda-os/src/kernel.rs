use common::ado_base::AndromedaQuery;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::messages::AMPPkt;

#[cw_serde]
pub struct InstantiateMsg {
    pub ibc_bridge: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Receives an AMP Packet for relaying
    AMPReceive(AMPPkt),
    /// Upserts a key address to the kernel, restricted to the owner of the kernel
    UpsertKeyAddress {
        key: String,
        value: String,
    },
    UpdateIbcBridge {
        new_address: String,
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
    KeyAddress { key: String },
    #[returns(bool)]
    VerifyAddress { address: String },
}
