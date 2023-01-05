use common::error::ContractError;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    entry_point, from_binary, to_binary, Addr, Binary, DepsMut, Empty, Env, IbcBasicResponse,
    IbcPacket, IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, IbcTimeout, StdResult,
    SubMsg, WasmMsg,
};
use cw721_proxy_derive::cw721_proxy;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct UniversalNftInfoResponse {
    pub token_uri: Option<String>,

    #[serde(skip_deserializing)]
    #[allow(dead_code)]
    extension: Empty,
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{from_binary, to_binary, Coin, Empty};

    use super::UniversalNftInfoResponse;

    #[test]
    fn test_universal_deserialize() {
        let start = cw721::NftInfoResponse::<Coin> {
            token_uri: None,
            extension: Coin::new(100, "ujuno"),
        };
        let start = to_binary(&start).unwrap();
        let end: UniversalNftInfoResponse = from_binary(&start).unwrap();
        assert_eq!(end.token_uri, None);
        assert_eq!(end.extension, Empty::default())
    }
}

#[cw_serde]
pub struct InstantiateMsg {}

#[cw721_proxy]
#[cw_serde]
pub enum ExecuteMsg {
    /// Receives a NFT to be IBC transfered away. The `outgoing_msg` field must
    /// be a binary encoded `IbcOutgoingMsg`.
    ReceiveMessage {
        outgoing_msg: Binary,
        user_msg: Binary,
    },
    /// Mesages used internally by the contract. These may only be
    /// called by the contract itself.
    Callback(CallbackMsg),
}

#[cw_serde]
pub enum CallbackMsg {
    HandlePacketReceive {
        /// The target contract's address.
        receiver: String,
        /// The message for the target contract.
        msg: Binary,
    },
}

#[cw_serde]
pub struct IbcOutgoingMsg {
    /// The contract address that should receive the message on the target chain
    pub receiver: String,
    /// The *local* channel ID this ought to be sent away on. This
    /// contract must have a connection on this channel.
    pub channel_id: String,
    /// Timeout for the IBC message.
    pub timeout: IbcTimeout,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    AuthorizedUser {},
}

#[cw_serde]
pub enum MigrateMsg {
    WithUpdate {
        /// The address that may pause the contract. If `None` is
        /// provided the current pauser will be removed.
        pauser: Option<String>,
        /// The cw721-proxy for this contract. If `None` is provided
        /// the current proxy will be removed.
        proxy: Option<String>,
    },
}

#[cw_serde]
pub enum Ack {
    Result(Binary),
    Error(String),
}

pub fn make_ack_fail(err: String) -> Binary {
    let res = Ack::Error(err);
    to_binary(&res).unwrap()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_receive(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, ContractError> {
    // Regardless of if our processing of this packet works we need to
    // commit an ACK to the chain. As such, we wrap all handling logic
    // in a seprate function and on error write out an error ack.
    match do_ibc_packet_receive(deps, env, msg.packet) {
        Ok(response) => Ok(response),
        Err(error) => Ok(IbcReceiveResponse::new()
            .add_attribute("method", "ibc_packet_receive")
            .add_attribute("error", error.to_string())
            .set_ack(make_ack_fail(error.to_string()))),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_timeout(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse, ContractError> {
    handle_packet_fail(deps, msg.packet, "timeout")
}

fn handle_packet_fail(
    _deps: DepsMut,
    packet: IbcPacket,
    error: &str,
) -> Result<IbcBasicResponse, ContractError> {
    let message: MessageBridgePacketData = from_binary(&packet.data)?;
    let target = message.target;
    let sender = message.sender;
    let user_msg = message.message;

    Ok(IbcBasicResponse::new()
        .add_attribute("method", "handle_packet_fail")
        .add_attribute("target_contract", target)
        .add_attribute("original_sender", sender)
        .add_attribute("channel_id", packet.src.channel_id)
        .add_attribute("user_msg", user_msg.to_string())
        .add_attribute("error", error))
}

pub fn do_ibc_packet_receive(
    deps: DepsMut,
    env: Env,
    packet: IbcPacket,
) -> Result<IbcReceiveResponse, ContractError> {
    let packet_data: MessageBridgePacketData = from_binary(&packet.data)?;

    // The address of the target bridge contract
    let contract = env.contract.address;

    // The address of the contract we're sending a message to
    let receiver = deps.api.addr_validate(&packet_data.target)?;

    // The message we're sending
    let msg = packet_data.clone().message;

    let submessage = into_submessage(contract, receiver, msg)?;

    Ok(IbcReceiveResponse::default()
        .add_submessage(submessage)
        .add_attribute("method", "do_ibc_packet_receive")
        .add_attribute("local_channel", packet.dest.channel_id)
        .add_attribute("counterparty_channel", packet.src.channel_id)
        .add_attribute("target_contract", packet_data.target)
        .add_attribute("message_sent_to_target", packet_data.message.to_string())
        .add_attribute("original_sender", packet_data.sender))
}

#[cw_serde]
pub struct MessageBridgePacketData {
    /// The address of the contract we're trying to messsage
    pub target: String,
    /// The message we're trying to send to the target contract
    pub message: Binary,
    /// The address that initiated the transction from the chain of origin.
    pub sender: String,
}

fn into_submessage(contract: Addr, receiver: Addr, msg: Binary) -> StdResult<SubMsg<Empty>> {
    Ok(SubMsg::reply_always(
        WasmMsg::Execute {
            contract_addr: contract.into_string(),
            msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::HandlePacketReceive {
                receiver: receiver.into_string(),
                msg,
            }))?,
            funds: vec![],
        },
        1,
    ))
}
