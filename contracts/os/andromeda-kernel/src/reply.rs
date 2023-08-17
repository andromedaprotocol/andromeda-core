use crate::{
    proto::MsgTransferResponse,
    state::{
        IBCHooksPacketSendState, OutgoingPacket, ADO_OWNER, OUTGOING_IBC_HOOKS_PACKETS,
        OUTGOING_IBC_PACKETS,
    },
};
use andromeda_std::{
    ado_base::AndromedaMsg, common::response::get_reply_address, error::ContractError,
};
use cosmwasm_std::{
    ensure, to_binary, wasm_execute, DepsMut, Empty, Reply, Response, SubMsg, SubMsgResponse,
    SubMsgResult,
};
use enum_repr::EnumRepr;

#[EnumRepr(type = "u64")]
pub enum ReplyId {
    AMPMsg = 1,
    CreateADO = 2,
    UpdateOwnership = 3,
    IBCHooksPacketSend = 4,
}

/// Handles the reply from an ADO creation
///
/// Sends an execute message to assign the new owner to the ADO. Will error if the owner is assigned in the ADO creation message.
pub fn on_reply_create_ado(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let new_owner = ADO_OWNER.load(deps.as_ref().storage)?;
    let ado_addr = get_reply_address(msg)?;

    let msg = AndromedaMsg::UpdateOwner {
        address: new_owner.to_string(),
    };
    let wasm_msg = wasm_execute(ado_addr.clone(), &msg, vec![])?;
    let sub_msg: SubMsg<Empty> =
        SubMsg::reply_on_success(wasm_msg, ReplyId::UpdateOwnership as u64);
    Ok(Response::default()
        .add_submessage(sub_msg)
        .set_data(to_binary(&ado_addr)?))
}

use ::prost::Message;
/// Adapted from https://github.com/osmosis-labs/osmosis/blob/main/cosmwasm/contracts/crosschain-swaps/src/execute.rs#L301
///
/// Handles the reply from sending an IBC hooks packet and creates an appropriate recovery
pub fn on_reply_ibc_hooks_packet_send(
    deps: DepsMut,
    msg: Reply,
) -> Result<Response, ContractError> {
    let SubMsgResult::Ok(SubMsgResponse { data: Some(b), .. }) = msg.result else {
        return Err(ContractError::InvalidPacket { error: Some(format!("ibc hooks: failed reply: {:?}", msg.result)) })
    };

    let MsgTransferResponse { sequence } =
        MsgTransferResponse::decode(&b[..]).map_err(|_e| ContractError::InvalidPacket {
            error: Some(format!("ibc hooks: could not decode response: {b}")),
        })?;

    let mut outgoing_packets = OUTGOING_IBC_HOOKS_PACKETS
        .load(deps.as_ref().storage)
        .unwrap_or_default();
    ensure!(
        !outgoing_packets.is_empty(),
        ContractError::InvalidPacket {
            error: Some(format!("ibc hooks: no outgoing packets"))
        }
    );

    let IBCHooksPacketSendState {
        channel_id,
        recovery_addr,
        amount,
    } = outgoing_packets.remove(0);

    OUTGOING_IBC_HOOKS_PACKETS.save(deps.storage, &outgoing_packets)?;
    OUTGOING_IBC_PACKETS.save(
        deps.storage,
        (&channel_id, sequence),
        &OutgoingPacket {
            recovery_addr: recovery_addr.clone(),
            amount,
        },
    )?;

    Ok(Response::default()
        .add_attribute("action", "ibc_hooks_packet_send")
        .add_attribute("channel_id", channel_id)
        .add_attribute("sequence", sequence.to_string())
        .add_attribute("recovery_addr", recovery_addr))
}
