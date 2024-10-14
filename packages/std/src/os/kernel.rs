use crate::ado_base::ownership::OwnershipMessage;
use crate::amp::messages::AMPMsg;
use crate::amp::messages::AMPPkt;
use crate::amp::AndrAddr;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use cosmwasm_std::Binary;
use cosmwasm_std::Coin;

#[cw_serde]
pub struct ChannelInfo {
    pub kernel_address: String,
    pub ics20_channel_id: Option<String>,
    pub direct_channel_id: Option<String>,
    pub supported_modules: Vec<String>,
}

impl Default for ChannelInfo {
    fn default() -> Self {
        ChannelInfo {
            kernel_address: "".to_string(),
            ics20_channel_id: None,
            direct_channel_id: None,
            supported_modules: vec![],
        }
    }
}

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    pub chain_name: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Receives an AMP Packet for relaying
    #[serde(rename = "amp_receive")]
    AMPReceive(AMPPkt),
    /// Constructs an AMPPkt with a given AMPMsg and sends it to the recipient
    Send {
        message: AMPMsg,
    },
    // TransferReply {
    //     message: AMPMsg,
    // },
    /// Upserts a key address to the kernel, restricted to the owner of the kernel
    UpsertKeyAddress {
        key: String,
        value: String,
    },
    /// Creates an ADO with the given type and message
    Create {
        ado_type: String,
        msg: Binary,
        owner: Option<AndrAddr>,
        chain: Option<String>,
    },
    /// Assigns a given channel to the given chain
    AssignChannels {
        ics20_channel_id: Option<String>,
        direct_channel_id: Option<String>,
        chain: String,
        kernel_address: String,
    },
    /// Recovers funds from failed IBC messages
    Recover {},
    /// Update Current Chain
    UpdateChainName {
        chain_name: String,
    },
    // Only accessible to key contracts
    Internal(InternalMsg),
    // Base message
    Ownership(OwnershipMessage),
}

#[cw_serde]
pub enum InternalMsg {
    // Restricted to VFS
    RegisterUserCrossChain {
        username: String,
        address: String,
        chain: String,
    },
}

#[cw_serde]
pub struct ChannelInfoResponse {
    pub ics20: Option<String>,
    pub direct: Option<String>,
    pub kernel_address: String,
    pub supported_modules: Vec<String>,
}

#[cw_serde]
pub struct ChainNameResponse {
    pub chain_name: String,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(cosmwasm_std::Addr)]
    KeyAddress { key: String },
    #[returns(VerifyAddressResponse)]
    VerifyAddress { address: String },
    #[returns(Option<ChannelInfoResponse>)]
    ChannelInfo { chain: String },
    #[returns(Vec<::cosmwasm_std::Coin>)]
    Recoveries { addr: Addr },
    #[returns(ChainNameResponse)]
    ChainName {},
    // Base queries
    #[returns(crate::ado_base::version::VersionResponse)]
    Version {},
    #[returns(crate::ado_base::ado_type::TypeResponse)]
    Type {},
    #[returns(crate::ado_base::ownership::ContractOwnerResponse)]
    Owner {},
}

#[cw_serde]
pub struct VerifyAddressResponse {
    pub verify_address: bool,
}

#[cw_serde]
pub enum IbcExecuteMsg {
    SendMessage {
        recipient: AndrAddr,
        message: Binary,
    },
    SendMessageWithFunds {
        recipient: AndrAddr,
        message: Binary,
        funds: Coin,
    },
    CreateADO {
        instantiation_msg: Binary,
        owner: AndrAddr,
        ado_type: String,
    },
    RegisterUsername {
        username: String,
        address: String,
    },
}
