use andromeda_os::messages::AMPMsg;
use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;

#[cw_serde]
pub struct InstantiateMsg {
    pub kernel_address: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    SendMessage {
        chain: String,
        recipient: String,
        message: Binary,
    },
    SendAmpPacket {
        chain: String,
        message: Vec<AMPMsg>,
    },
    SaveChannel {
        channel: String,
        chain: String,
    },
    UpdateChannel {
        channel: String,
        chain: String,
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
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    #[returns(String)]
    ChannelID { chain: String },
    #[returns(Vec<String>)]
    SupportedChains {},
}
