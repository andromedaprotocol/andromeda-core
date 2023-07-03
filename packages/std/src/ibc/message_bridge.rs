use crate::ado_base::{AndromedaMsg, AndromedaQuery};
use crate::amp::messages::{AMPMsg, AMPPkt};
use crate::amp::AndrAddr;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;

#[cw_serde]
pub struct InstantiateMsg {
    pub kernel_address: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    AMPReceive(AMPPkt),
    SendMessage {
        chain: String,
        recipient: AndrAddr,
        message: Binary,
    },
    SendAmpPacket {
        chain: String,
        message: Vec<AMPMsg>,
    },
    SaveChannel {
        channel: String,
        chain: String,
        kernel_address: String,
    },
    UpdateChannel {
        channel: String,
        chain: String,
        kernel_address: Option<String>,
    },
}

#[cw_serde]
pub enum IbcExecuteMsg {
    SendMessage { recipient: String, message: Binary },
    SendAmpPacket { message: Binary },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(String)]
    ChannelID { chain: String },
    #[returns(Vec<String>)]
    SupportedChains {},
}
