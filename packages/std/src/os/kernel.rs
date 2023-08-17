use crate::amp::messages::AMPMsg;
use crate::amp::messages::AMPPkt;
use crate::amp::AndrAddr;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use cosmwasm_std::Binary;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    pub chain_name: Option<String>,
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
    /// Assigns a given channel to the given chain
    AssignChannels {
        ics20_channel_id: Option<String>,
        direct_channel_id: Option<String>,
        chain: String,
        kernel_address: String,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct ChannelInfoResponse {
    pub ics20: Option<String>,
    pub direct: Option<String>,
    pub kernel_address: String,
    pub supported_modules: Vec<String>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(cosmwasm_std::Addr)]
    KeyAddress { key: String },
    #[returns(bool)]
    VerifyAddress { address: String },
    #[returns(Option<ChannelInfoResponse>)]
    ChannelInfo { chain: String },
    #[returns(Vec<::cosmwasm_std::Coin>)]
    Recoveries { addr: Addr },
}

#[cw_serde]
pub enum IbcExecuteMsg {
    SendMessage {
        recipient: AndrAddr,
        message: Binary,
    },
}
