use crate::amp::messages::AMPMsg;
use crate::amp::messages::AMPPkt;
use crate::amp::AndrAddr;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Receives an AMP Packet for relaying
    #[serde(rename = "amp_receive")]
    AMPReceive(AMPPkt),
    /// Constructs an AMPPkt with a given AMPMsg and sends it to the recipient
    Send { message: AMPMsg },
    /// Upserts a key address to the kernel, restricted to the owner of the kernel
    UpsertKeyAddress { key: String, value: String },
    /// Creates an ADO with the given type and message
    Create {
        ado_type: String,
        msg: Binary,
        owner: Option<AndrAddr>,
    },
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
