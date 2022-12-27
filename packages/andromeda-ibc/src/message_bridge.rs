use common::error::ContractError;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, CosmosMsg, DepsMut, Empty, Env, IbcPacket,
    IbcPacketReceiveMsg, IbcReceiveResponse, IbcTimeout, WasmMsg,
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
    /// Receives a NFT to be IBC transfered away. The `msg` field must
    /// be a binary encoded `IbcOutgoingMsg`.
    ReceiveMessage {
        target: String,
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

pub fn do_ibc_packet_receive(
    _deps: DepsMut,
    _env: Env,
    packet: IbcPacket,
) -> Result<IbcReceiveResponse, ContractError> {
    let packet_data: MessageBridgePacketData = from_binary(&packet.data)?;

    Ok(IbcReceiveResponse::default()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: packet_data.clone().target,
            msg: packet_data.clone().message,
            funds: vec![],
        }))
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
