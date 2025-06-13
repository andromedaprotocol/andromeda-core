use crate::{
    ibc::PACKET_LIFETIME,
    proto::MsgTransferResponse,
    state::{ADO_OWNER, CHANNEL_TO_EXECUTE_MSG, PENDING_MSG_AND_FUNDS, REFUND_DATA},
};
use andromeda_std::{
    ado_base::{ownership::OwnershipMessage, AndromedaMsg},
    common::reply::ReplyId,
    common::response::get_reply_address,
    error::ContractError,
    os::aos_querier::AOSQuerier,
};
use cosmwasm_std::{
    to_json_string, wasm_execute, Addr, CosmosMsg, DepsMut, Empty, Env, IbcMsg, Reply, Response,
    SubMsg, SubMsgResponse, SubMsgResult,
};
use prost::Message;

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

// Handles the reply from an ICS20 funds transfer that did inlcude a message
#[allow(deprecated)]
pub fn on_reply_ibc_transfer(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    // TODO this is deprecated
    if let SubMsgResult::Ok(SubMsgResponse { data: Some(b), .. }) = msg.result {
        let MsgTransferResponse { sequence } =
            MsgTransferResponse::decode(&b[..]).map_err(|_e| ContractError::InvalidPacket {
                error: Some(format!("could not decode response: {b}")),
            })?;

        let pending_execute_msg = PENDING_MSG_AND_FUNDS.load(deps.storage)?;
        CHANNEL_TO_EXECUTE_MSG.save(
            deps.storage,
            (pending_execute_msg.channel.clone(), sequence),
            &pending_execute_msg,
        )?;
        PENDING_MSG_AND_FUNDS.remove(deps.storage);
        return Ok(Response::new()
            .add_attribute("action", "transfer_funds_reply")
            .add_attribute("sequence", sequence.to_string()));
    }

    #[cfg(not(target_arch = "wasm32"))]
    // When the reply is from a non-wasm32 target, the reply data is pulled from the events
    if let Reply {
        id: 106,
        result: SubMsgResult::Ok(SubMsgResponse { events, .. }),
        payload: _,
        gas_used: _,
    } = msg.clone()
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
                (
                    pending_execute_msg.channel.clone(),
                    packet_sequence.parse().unwrap(),
                ),
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
        .add_attribute("amount_refunded", refund_coin.to_string())
        .add_attribute("reply", to_json_string(&msg)?))
}

// Handles the reply from an Execute Msg that was preceded by an ICS20 transfer
pub fn on_reply_refund_ibc_transfer_with_msg(
    deps: DepsMut,
    env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    let err = msg.result.unwrap_err();
    let refund_data = REFUND_DATA.load(deps.storage)?;
    // Construct the refund message
    let refund_msg = IbcMsg::Transfer {
        channel_id: refund_data.channel,
        to_address: refund_data.original_sender.clone(),
        amount: refund_data.funds.clone(),
        timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
        memo: None,
    };
    REFUND_DATA.remove(deps.storage);
    Ok(Response::default()
        .add_message(refund_msg)
        .add_attributes(vec![
            ("action", "refund_ibc_transfer_with_msg"),
            ("recipient", refund_data.original_sender.as_str()),
            ("amount", refund_data.funds.to_string().as_str()),
            ("error", err.to_string().as_str()),
        ]))
}
