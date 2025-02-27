use crate::{
    ado_base::ownership::OwnershipMessage,
    amp::{
        messages::{AMPMsg, AMPPkt, CrossChainHop},
        AndrAddr,
    },
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Coin};
use cw20::Cw20ReceiveMsg;

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
#[cfg_attr(not(target_arch = "wasm32"), derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    /// Receives an AMP Packet for relaying
    #[serde(rename = "amp_receive")]
    AMPReceive(AMPPkt),
    // Cw20 entry point
    Receive(Cw20ReceiveMsg),
    /// Constructs an AMPPkt with a given AMPMsg and sends it to the recipient
    Send {
        message: AMPMsg,
    },
    TriggerRelay {
        packet_sequence: u64,
        channel_id: String,
        packet_ack: Binary,
    },
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
    /// Sets an environment variable with the given name and value.
    /// The variable name must be uppercase and can only contain letters, numbers, and underscores.
    /// The value must be a valid UTF-8 string.
    SetEnv {
        variable: String,
        value: String,
    },
    /// Removes an environment variable with the given name.
    /// Returns success even if the variable doesn't exist.
    UnsetEnv {
        variable: String,
    },
    // Only accessible to key contracts
    Internal(InternalMsg),
    // Base message
    Ownership(OwnershipMessage),
}

#[cw_serde]
pub enum Cw20HookMsg {
    AmpReceive(AMPPkt),
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
pub struct PendingPacketResponse {
    pub packets: Vec<PacketInfoAndSequence>,
}

#[cw_serde]
pub struct PacketInfoAndSequence {
    pub packet_info: Ics20PacketInfo,
    pub sequence: u64,
}

#[cw_serde]
pub struct EnvResponse {
    pub value: Option<String>,
}

#[cw_serde]
#[cfg_attr(not(target_arch = "wasm32"), derive(cw_orch::QueryFns))]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(cosmwasm_std::Addr)]
    KeyAddress { key: String },
    #[returns(VerifyAddressResponse)]
    VerifyAddress { address: String },
    #[returns(Option<ChannelInfoResponse>)]
    ChannelInfo { chain: String },
    #[returns(Option<String>)]
    ChainNameByChannel { channel: String },
    #[returns(Vec<::cosmwasm_std::Coin>)]
    Recoveries { addr: Addr },
    #[returns(ChainNameResponse)]
    ChainName {},
    // Base queries
    #[returns(crate::ado_base::version::VersionResponse)]
    Version {},
    #[returns(crate::ado_base::ado_type::TypeResponse)]
    #[serde(rename = "type")]
    AdoType {},
    #[returns(crate::ado_base::ownership::ContractOwnerResponse)]
    Owner {},
    #[returns(PendingPacketResponse)]
    PendingPackets { channel_id: Option<String> },
    #[returns(EnvResponse)]
    GetEnv { variable: String },
}

#[cw_serde]
pub struct VerifyAddressResponse {
    pub verify_address: bool,
}

#[cw_serde]
pub enum IbcExecuteMsg {
    SendMessage {
        amp_packet: AMPPkt,
    },
    SendMessageWithFunds {
        recipient: AndrAddr,
        message: Binary,
        funds: Coin,
        original_sender: String,
        original_sender_username: Option<AndrAddr>,
        previous_hops: Vec<CrossChainHop>,
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

#[cw_serde]
pub struct Ics20PacketInfo {
    // Can be used for refunds in case the first Transfer msg fails
    pub sender: String,
    pub recipient: AndrAddr,
    pub message: Binary,
    pub funds: Coin,
    // The restricted wallet will probably already have access to this
    pub channel: String,
    pub pending: bool,
}

#[cw_serde]
pub struct RefundData {
    pub original_sender: String,
    pub funds: Coin,
    pub channel: String,
}

use crate::common::reply::ReplyId;
use crate::error::ContractError;
use cosmwasm_std::{attr, BankMsg, CosmosMsg, DepsMut, SubMsg};
/// Creates a bank send message and its associated attributes for transferring funds to a recipient.
///
/// # Arguments
///
/// * `recipient` - The address that will receive the funds
/// * `funds` - A slice of `Coin`s representing the funds to be sent
///
/// # Returns
///
/// Returns a tuple containing:
/// * `SubMsg` - A submessage configured to send the funds with error reply handling
/// * `Vec<Attribute>` - A vector of attributes documenting the funds transfer, including:
///   * One attribute per coin showing the amount and denomination
///   * One attribute showing the recipient address
pub fn create_bank_send_msg(
    recipient: &Addr,
    funds: &[Coin],
) -> (SubMsg, Vec<cosmwasm_std::Attribute>) {
    let bank_msg = SubMsg::reply_on_error(
        CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient.to_string(),
            amount: funds.to_vec(),
        }),
        ReplyId::AMPMsg.repr(),
    );

    let attrs = funds
        .iter()
        .enumerate()
        .map(|(idx, fund)| attr(format!("funds:{idx}"), fund.to_string()))
        .chain(std::iter::once(attr("recipient", recipient.to_string())))
        .collect();

    (bank_msg, attrs)
}

/// Retrieves the code ID for a given contract address.
///
/// This function verifies that the provided address is a valid smart contract by querying
/// its contract info. If the address is not a contract, it returns an error.
///
/// # Arguments
///
/// * `deps` - A reference to the contract's dependencies, used for querying contract info
/// * `recipient` - The address to check and get the code ID for
///
/// # Returns
///
/// * `Result<u64, ContractError>` - The code ID if successful, or a ContractError if:
///   * The address is not a contract
///   * The query fails
pub fn get_code_id(deps: &DepsMut, recipient: &AndrAddr) -> Result<u64, ContractError> {
    deps.querier
        .query_wasm_contract_info(recipient.get_raw_address(&deps.as_ref())?)
        .ok()
        .ok_or(ContractError::InvalidPacket {
            error: Some("Recipient is not a contract".to_string()),
        })
        .map(|info| info.code_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(
        Addr::unchecked("recipient"),
        vec![Coin::new(100, "uusd")],
        1
    )]
    #[case(
        Addr::unchecked("recipient2"),
        vec![Coin::new(100, "uusd"), Coin::new(200, "uluna")],
        2
    )]
    #[case(
        Addr::unchecked("recipient3"),
        vec![],
        0
    )]
    fn test_create_bank_send_msg(
        #[case] recipient: Addr,
        #[case] funds: Vec<Coin>,
        #[case] expected_fund_attrs: usize,
    ) {
        let (submsg, attrs) = create_bank_send_msg(&recipient, &funds);

        // Check the SubMsg
        match submsg.msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(to_address, recipient.to_string());
                assert_eq!(amount, funds);
            }
            _ => panic!("Expected BankMsg::Send"),
        }
        assert_eq!(submsg.id, ReplyId::AMPMsg.repr());
        assert_eq!(submsg.reply_on, cosmwasm_std::ReplyOn::Error);

        // Check attributes
        assert_eq!(attrs.len(), expected_fund_attrs + 1); // +1 for recipient attribute
        assert_eq!(attrs.last().unwrap().key, "recipient");
        assert_eq!(attrs.last().unwrap().value, recipient.to_string());

        // Check fund attributes
        for (idx, fund) in funds.iter().enumerate() {
            assert_eq!(attrs[idx].key, format!("funds:{idx}"));
            assert_eq!(attrs[idx].value, fund.to_string());
        }
    }
}
