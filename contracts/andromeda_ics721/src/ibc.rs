use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    attr, entry_point, from_binary, to_binary, Binary, DepsMut, Env, IbcBasicResponse, IbcChannel,
    IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcOrder, IbcPacket,
    IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, SubMsg, WasmMsg,
};

use crate::error::{ContractError, Never};
use crate::state::{ChannelInfo, CHANNEL_INFO};
use andromeda_protocol::token::NftInfoResponseExtension;
use cw721::{Cw721ExecuteMsg, NftInfoResponse};

pub const ICS721_VERSION: &str = "ics721-1";
pub const ICS721_ORDERING: IbcOrder = IbcOrder::Unordered;

/// The format for sending an ics721 packet.
/// This is compatible with the JSON serialization
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Ics721Packet {
    pub token_id: String,
    pub token_addr: String,
    pub token_info: NftInfoResponse<NftInfoResponseExtension>,
    /// the recipient address on the destination chain
    pub receiver: String,
    /// the sender address
    pub sender: String,
}

impl Ics721Packet {
    pub fn new(
        token_id: String,
        token_addr: String,
        token_info: NftInfoResponse<NftInfoResponseExtension>,
        sender: &str,
        receiver: &str,
    ) -> Self {
        Ics721Packet {
            token_id,
            token_addr,
            token_info,
            sender: sender.to_string(),
            receiver: receiver.to_string(),
        }
    }
}

/// This is a generic ICS acknowledgement format.
/// Proto defined here: https://github.com/cosmos/cosmos-sdk/blob/v0.42.0/proto/ibc/core/channel/v1/channel.proto#L141-L147
/// This is compatible with the JSON serialization
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Ics721Ack {
    Result(Binary),
    Error(String),
}

// create a serialized success message
fn ack_success() -> Binary {
    let res = Ics721Ack::Result(b"1".into());
    to_binary(&res).unwrap()
}

// create a serialized error message
fn ack_fail(err: String) -> Binary {
    let res = Ics721Ack::Error(err);
    to_binary(&res).unwrap()
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// enforces ordering and versioning constraints
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<(), ContractError> {
    enforce_order_and_version(msg.channel(), msg.counterparty_version())?;
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// record the channel in CHANNEL_INFO
pub fn ibc_channel_connect(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // we need to check the counter party version in try and ack (sometimes here)
    enforce_order_and_version(msg.channel(), msg.counterparty_version())?;

    let channel: IbcChannel = msg.into();
    let info = ChannelInfo {
        id: channel.endpoint.channel_id,
        counterparty_endpoint: channel.counterparty_endpoint,
        connection_id: channel.connection_id,
    };
    CHANNEL_INFO.save(deps.storage, &info.id, &info)?;

    Ok(IbcBasicResponse::default())
}

fn enforce_order_and_version(
    channel: &IbcChannel,
    counterparty_version: Option<&str>,
) -> Result<(), ContractError> {
    if channel.version != ICS721_VERSION {
        return Err(ContractError::InvalidIbcVersion {
            version: channel.version.clone(),
        });
    }
    if let Some(version) = counterparty_version {
        if version != ICS721_VERSION {
            return Err(ContractError::InvalidIbcVersion {
                version: version.to_string(),
            });
        }
    }
    if channel.order != ICS721_ORDERING {
        return Err(ContractError::OnlyOrderedChannel {});
    }
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    _channel: IbcChannelCloseMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // TODO: what to do here?
    // we will have locked funds that need to be returned somehow
    unimplemented!();
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// Check to see if we have any balance here
/// We should not return an error if possible, but rather an acknowledgement of failure
pub fn ibc_packet_receive(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, Never> {
    let packet = msg.packet;

    let res = match do_ibc_packet_receive(deps, &packet) {
        Ok(msg) => {
            // build attributes first so we don't have to clone msg below
            // similar event messages like ibctransfer module

            // This cannot fail as we parse it in do_ibc_packet_receive. Best to pass the data somehow?

            let attributes = vec![
                attr("action", "receive"),
                attr("sender", &msg.sender),
                attr("receiver", &msg.receiver),
                attr("success", "true"),
            ];
            let token_id = msg.token_id;
            let token_addr = msg.token_addr;
            let msg = send_token(token_id, token_addr, msg.receiver);
            IbcReceiveResponse::new()
                .set_ack(ack_success())
                .add_submessage(msg)
                .add_attributes(attributes)
        }
        Err(err) => IbcReceiveResponse::new()
            .set_ack(ack_fail(err.to_string()))
            .add_attributes(vec![
                attr("action", "receive"),
                attr("success", "false"),
                attr("error", err.to_string()),
            ]),
    };

    // if we have funds, now send the tokens to the requested recipient
    Ok(res)
}

// this does the work of ibc_packet_receive, we wrap it to turn errors into acknowledgements
fn do_ibc_packet_receive(
    _deps: DepsMut,
    packet: &IbcPacket,
) -> Result<Ics721Packet, ContractError> {
    let msg: Ics721Packet = from_binary(&packet.data)?;

    Ok(msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// check if success or failure and update balance, or return funds
pub fn ibc_packet_ack(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // TODO: trap error like in receive?
    let ics721msg: Ics721Ack = from_binary(&msg.acknowledgement.data)?;
    match ics721msg {
        Ics721Ack::Result(_) => on_packet_success(deps, msg.original_packet),
        Ics721Ack::Error(err) => on_packet_failure(deps, msg.original_packet, err),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// return fund to original sender (same as failure in ibc_packet_ack)
pub fn ibc_packet_timeout(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // TODO: trap error like in receive?
    let packet = msg.packet;
    on_packet_failure(deps, packet, "timeout".to_string())
}

// update the balance stored on this (channel, denom) index
fn on_packet_success(_deps: DepsMut, packet: IbcPacket) -> Result<IbcBasicResponse, ContractError> {
    let msg: Ics721Packet = from_binary(&packet.data)?;
    // similar event messages like ibctransfer module
    let attributes = vec![
        attr("action", "acknowledge"),
        attr("sender", &msg.sender),
        attr("receiver", &msg.receiver),
        attr("success", "true"),
    ];

    Ok(IbcBasicResponse::new().add_attributes(attributes))
}

// return the tokens to sender
fn on_packet_failure(
    _deps: DepsMut,
    packet: IbcPacket,
    err: String,
) -> Result<IbcBasicResponse, ContractError> {
    let msg: Ics721Packet = from_binary(&packet.data)?;
    // similar event messages like ibctransfer module
    let attributes = vec![
        attr("action", "acknowledge"),
        attr("sender", &msg.sender),
        attr("receiver", &msg.receiver),
        attr("success", "false"),
        attr("error", err),
    ];

    let msg = send_token(msg.token_id, msg.token_addr, msg.sender);
    Ok(IbcBasicResponse::new()
        .add_attributes(attributes)
        .add_submessage(msg))
}

fn send_token(token_id: String, token_addr: String, recipient: String) -> SubMsg {
    let msg = Cw721ExecuteMsg::TransferNft {
        recipient,
        token_id,
    };
    let exec = WasmMsg::Execute {
        contract_addr: token_addr,
        msg: to_binary(&msg).unwrap(),
        funds: vec![],
    };
    SubMsg::new(exec)
}
