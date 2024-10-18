use crate::{
    proto::MsgTransferResponse,
    state::{
        IBCHooksPacketSendState, OutgoingPacket, ADO_OWNER, CHANNEL_TO_EXECUTE_MSG,
        OUTGOING_IBC_HOOKS_PACKETS, OUTGOING_IBC_PACKETS, PENDING_MSG_AND_FUNDS,
    },
};
use andromeda_std::{
    ado_base::{ownership::OwnershipMessage, AndromedaMsg},
    common::reply::ReplyId,
    common::response::get_reply_address,
    error::ContractError,
    os::aos_querier::AOSQuerier,
};
use cosmwasm_std::{
    ensure, wasm_execute, Addr, CosmosMsg, DepsMut, Empty, Env, Reply, Response, SubMsg,
    SubMsgResponse, SubMsgResult,
};

/// Handles the reply from an ADO creation
///
/// Sends an execute message to assign the new owner to the ADO
pub fn on_reply_create_ado(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    let new_owner = ADO_OWNER.load(deps.as_ref().storage)?;
    let ado_addr = get_reply_address(msg)?;

    let curr_owner =
        AOSQuerier::ado_owner_getter(&deps.querier, &Addr::unchecked(ado_addr.clone()))?;
    let mut res = Response::default();
    if curr_owner == env.contract.address {
        let msg = AndromedaMsg::Ownership(OwnershipMessage::UpdateOwner {
            new_owner,
            expiration: None,
        });
        let wasm_msg = wasm_execute(ado_addr, &msg, vec![])?;
        let sub_msg: SubMsg<Empty> =
            SubMsg::reply_on_success(wasm_msg, ReplyId::UpdateOwnership as u64);
        res = res.add_submessage(sub_msg);
    }

    Ok(res)
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
        return Err(ContractError::InvalidPacket {
            error: Some(format!("ibc hooks: failed reply: {:?}", msg.result)),
        });
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
            error: Some("ibc hooks: no outgoing packets".to_string())
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

// Handles the reply from an ICS20 funds transfer that did inlcude a message
pub fn on_reply_ibc_transfer(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    if let Reply {
        id: 106,
        result: SubMsgResult::Ok(SubMsgResponse { events, .. }),
    } = msg
    {
        if let Some(send_packet_event) = events.iter().find(|e| e.ty == "send_packet") {
            let packet_data = send_packet_event
                .attributes
                .iter()
                .find(|attr| attr.key == "packet_data")
                .map(|attr| attr.value.clone())
                .unwrap_or_default();
            let packet_sequence = send_packet_event
                .attributes
                .iter()
                .find(|attr| attr.key == "packet_sequence")
                .map(|attr| attr.value.clone())
                .unwrap_or_default();
            let src_channel = send_packet_event
                .attributes
                .iter()
                .find(|attr| attr.key == "packet_src_channel")
                .map(|attr| attr.value.clone())
                .unwrap_or_default();
            let dst_channel = send_packet_event
                .attributes
                .iter()
                .find(|attr| attr.key == "packet_dst_channel")
                .map(|attr| attr.value.clone())
                .unwrap_or_default();
            let pending_execute_msg = PENDING_MSG_AND_FUNDS.load(deps.storage)?;
            CHANNEL_TO_EXECUTE_MSG.save(
                deps.storage,
                packet_sequence.clone(),
                &pending_execute_msg,
            )?;
            PENDING_MSG_AND_FUNDS.remove(deps.storage);
            // You can now use these extracted values as needed
            // For example, you might want to store them or include them in the response
            return Ok(Response::new()
                .add_attribute("action", "transfer_funds_reply")
                .add_attribute("packet_data", packet_data)
                .add_attribute("packet_sequence", packet_sequence)
                .add_attribute("src_channel", src_channel)
                .add_attribute("dst_channel", dst_channel));
        }
    }
    // Refund original message sender
    let ics20_packet_info = PENDING_MSG_AND_FUNDS.load(deps.storage)?;
    let refund_recipient = ics20_packet_info.sender;
    let refund_coin = ics20_packet_info.funds;
    let refund_msg = CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
        to_address: refund_recipient.clone(),
        amount: vec![refund_coin.clone()],
    });

    // Clear data
    PENDING_MSG_AND_FUNDS.remove(deps.storage);

    Ok(Response::default()
        .add_message(refund_msg)
        .add_attribute("action", "refund")
        .add_attribute("recipient", refund_recipient)
        .add_attribute("amount_refunded", refund_coin.to_string()))
}
