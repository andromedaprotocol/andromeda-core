use crate::amp::addresses::AndrAddr;
use crate::amp::messages::AMPPkt;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, ReplyOn};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    // #[serde(rename = "amp_receive")]
    /// Receives an AMP Packet for relaying
    AMPReceive(AMPPkt),
    /// Creates an original AMP packet
    AMPDirect {
        recipient: AndrAddr,
        message: Binary,
        reply_on: Option<ReplyOn>,
        exit_at_error: Option<bool>,
        gas_limit: Option<u64>,
    },
    /// Upserts a key address to the kernel, restricted to the owner of the kernel
    UpsertKeyAddress { key: String, value: String },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(cosmwasm_std::Addr)]
    KeyAddress { key: String },
    #[returns(bool)]
    VerifyAddress { address: String },
}
