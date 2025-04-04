use crate::ibc::{get_counterparty_denom, PACKET_LIFETIME};
use crate::query;
use crate::state::{
    ADO_OWNER, CHAIN_TO_CHANNEL, CHANNEL_TO_CHAIN, CHANNEL_TO_EXECUTE_MSG, CURR_CHAIN,
    ENV_VARIABLES, IBC_FUND_RECOVERY, KERNEL_ADDRESSES, PENDING_MSG_AND_FUNDS, TRIGGER_KEY,
    TX_INDEX,
};
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::amp::addresses::AndrAddr;
use andromeda_std::amp::messages::{AMPCtx, AMPMsg, AMPPkt, CrossChainHop};
use andromeda_std::amp::{ADO_DB_KEY, VFS_KEY};
use andromeda_std::common::code_id::get_code_id;
use andromeda_std::common::context::ExecuteContext;
use andromeda_std::common::has_coins_merged;
use andromeda_std::common::message_generators::{
    create_bank_send_msg, create_cw20_send_msg, create_cw20_transfer_msg,
};
use andromeda_std::common::reply::ReplyId;
use andromeda_std::error::ContractError;
use andromeda_std::os::aos_querier::AOSQuerier;
#[cfg(not(target_arch = "wasm32"))]
use andromeda_std::os::ibc_registry::path_to_hops;
use andromeda_std::os::kernel::{
    ChannelInfo, Cw20HookMsg, ExecuteMsg, IbcExecuteMsg, Ics20PacketInfo, InternalMsg,
};
use cosmwasm_std::{
    attr, ensure, from_json, to_json_binary, BankMsg, Binary, Coin, CosmosMsg, DepsMut, Env,
    IbcMsg, MessageInfo, Response, StdAck, StdError, SubMsg, Uint128, WasmMsg,
};
use cw20::Cw20ReceiveMsg;

pub fn send(ctx: ExecuteContext, message: AMPMsg) -> Result<Response, ContractError> {
    ensure!(
        has_coins_merged(ctx.info.funds.as_slice(), message.funds.as_slice()),
        ContractError::InsufficientFunds {}
    );

    handle(ctx.deps, ctx.info, ctx.env, ctx.amp_ctx, message)
}

pub fn handle(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    ctx: Option<AMPPkt>,
    message: AMPMsg,
) -> Result<Response, ContractError> {
    match message.recipient.get_protocol() {
        Some("ibc") => handle_ibc(deps, info, env, ctx, message),
        _ => handle_local(deps, info, env, ctx.map(|ctx| ctx.ctx), message),
    }
}

/**
Handles a local AMP Message, that is a message that has no defined protocol in its recipient VFS path. There are two different situations for a local message that are defined by the binary message provided.
Situation 1 is that the message provided is empty or `Binary::default` in which case the message must be a `BankMsg::Send` message and the funds must be provided.
Situation 2 is that the message has a provided binary and must be a `WasmMsg::Execute` message.

In both situations the sender can define the funds that are being attached to the message.
*/
pub fn handle_local(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    ctx: Option<AMPCtx>,
    amp_message: AMPMsg,
) -> Result<Response, ContractError> {
    let AMPMsg {
        ref message,
        ref funds,
        ref config,
        ref recipient,
    } = amp_message;

    let recipient_addr = recipient.get_raw_address(&deps.as_ref())?;

    // Handle empty message - send funds only
    if message == &Binary::default() {
        ensure!(
            !funds.is_empty(),
            ContractError::InvalidPacket {
                error: Some("No funds supplied".to_string())
            }
        );

        let (bank_msg, attrs) =
            create_bank_send_msg(&recipient_addr, funds, ReplyId::AMPMsg.repr());

        return Ok(Response::default()
            .add_submessage(bank_msg)
            .add_attributes(attrs));
    }

    // Get the ADODB address
    let adodb_addr = KERNEL_ADDRESSES.load(deps.storage, ADO_DB_KEY)?;

    // Verify recipient is a contract
    let code_id = get_code_id(&deps, recipient)?;
    // Check if the recipient is an ADO
    let is_ado = AOSQuerier::ado_type_getter(&deps.querier, &adodb_addr, code_id)?.is_some();

    // Generate submessage based on whether recipient is an ADO or if the message is direct
    let sub_msg = if config.direct || !is_ado {
        amp_message.generate_sub_msg_direct(recipient_addr, ReplyId::AMPMsg.repr())
    } else {
        let origin = ctx.map_or(info.sender.to_string(), |ctx| ctx.get_origin());
        let previous_sender = info.sender.to_string();

        AMPPkt::new(origin, previous_sender, vec![amp_message.clone()]).to_sub_msg(
            recipient_addr,
            Some(funds.clone()),
            ReplyId::AMPMsg.repr(),
        )?
    };

    Ok(Response::default()
        .add_submessage(sub_msg)
        .add_attribute("recipient", recipient))
}

pub fn handle_receive_cw20(
    ctx: ExecuteContext,
    receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        mut deps,
        info,
        env,
        amp_ctx,
        ..
    } = ctx;

    let asset_sent = info.sender.clone().into_string();
    let amount_sent = receive_msg.amount;

    match from_json(&receive_msg.msg)? {
        Cw20HookMsg::Send { message } => {
            ensure!(
                has_coins_merged(
                    vec![Coin::new(amount_sent.into(), &asset_sent)].as_slice(),
                    message.funds.as_slice()
                ),
                ContractError::InsufficientFunds {}
            );

            match message.recipient.get_protocol() {
                Some(_) => Err(ContractError::NotImplemented {
                    msg: Some("CW20 over IBC not supported".to_string()),
                }),
                _ => handle_local_cw20(deps, info, env, amp_ctx.map(|ctx| ctx.ctx), message),
            }
        }
        Cw20HookMsg::AmpReceive(packet) => amp_receive_cw20(
            &mut deps,
            info,
            env,
            packet,
            vec![Coin::new(amount_sent.into(), &asset_sent)],
        ),
    }
}

pub fn handle_local_cw20(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    ctx: Option<AMPCtx>,
    amp_message: AMPMsg,
) -> Result<Response, ContractError> {
    let res = Response::default();
    let AMPMsg {
        ref message,
        ref recipient,
        ref funds,
        ref config,
        ..
    } = amp_message;

    let token_denom = funds[0].denom.clone();
    let token_amount = funds[0].amount.u128();
    let recipient_raw_address = recipient.get_raw_address(&deps.as_ref())?;

    // Handle empty message (bank transfer)
    if message == &Binary::default() {
        ensure!(
            !funds.is_empty(),
            ContractError::InvalidPacket {
                error: Some("No funds supplied".to_string())
            }
        );

        let (sub_msg, attrs) = create_cw20_transfer_msg(
            &recipient_raw_address,
            &token_denom,
            token_amount,
            ReplyId::AMPMsg.repr(),
        )?;

        return Ok(res.add_submessage(sub_msg).add_attributes(attrs));
    }

    // Verify recipient is contract
    let recipient_code_id = get_code_id(&deps, recipient)?;
    let adodb_addr = KERNEL_ADDRESSES.load(deps.storage, ADO_DB_KEY)?;

    let is_ado =
        AOSQuerier::ado_type_getter(&deps.querier, &adodb_addr, recipient_code_id)?.is_some();

    let (sub_msg, attrs) = if config.direct || !is_ado {
        // Direct message
        create_cw20_send_msg(
            &recipient_raw_address,
            &token_denom,
            token_amount,
            message.clone(),
            config.clone(),
            ReplyId::AMPMsg.repr(),
        )?
    } else {
        let origin = ctx.map_or(info.sender.to_string(), |ctx| ctx.get_origin());
        let previous_sender = info.sender.to_string();

        let new_packet = AMPPkt::new(origin, previous_sender, vec![amp_message.clone()]);

        create_cw20_send_msg(
            &recipient_raw_address,
            &token_denom,
            token_amount,
            to_json_binary(&ExecuteMsg::AMPReceive(new_packet))?,
            config.clone(),
            ReplyId::AMPMsg.repr(),
        )?
    };

    Ok(res.add_submessage(sub_msg).add_attributes(attrs))
}

pub fn trigger_relay(
    ctx: ExecuteContext,
    packet_sequence: u64,
    channel_id: String,
    packet_ack_msg: Binary,
) -> Result<Response, ContractError> {
    //TODO Only the authorized address to handle replies can call this function
    ensure!(
        ctx.info.sender == KERNEL_ADDRESSES.load(ctx.deps.storage, TRIGGER_KEY)?,
        ContractError::Unauthorized {}
    );
    let ics20_packet_info = CHANNEL_TO_EXECUTE_MSG
        .load(ctx.deps.storage, (channel_id.clone(), packet_sequence))
        .expect("No packet found for channel_id and sequence");

    let chain = ics20_packet_info
        .recipient
        .get_chain()
        .ok_or(ContractError::InvalidPacket {
            error: Some("Chain not provided".to_string()),
        })?;

    let channel_info = CHAIN_TO_CHANNEL.may_load(ctx.deps.storage, chain)?.ok_or(
        ContractError::InvalidPacket {
            error: Some(format!("Channel not found for chain {}", chain)),
        },
    )?;
    let ack: StdAck = from_json(packet_ack_msg)?;

    match ack {
        StdAck::Success(_) => handle_ibc_transfer_funds_reply(
            ctx.deps,
            ctx.info,
            ctx.env,
            ctx.amp_ctx,
            packet_sequence,
            channel_info,
            ics20_packet_info,
            channel_id,
        ),
        // This means that the funds have been returned to the contract, time to return the funds to the original sender
        StdAck::Error(_) => {
            let refund_msg = CosmosMsg::Bank(BankMsg::Send {
                to_address: ics20_packet_info.sender.clone(),
                amount: vec![ics20_packet_info.funds.clone()],
            });
            Ok(Response::default()
                .add_message(refund_msg)
                .add_attribute("action", "relay_packet")
                .add_attribute("relay_outcome", "refund")
                .add_attribute("relay_sequence", packet_sequence.to_string())
                .add_attribute("relay_channel", channel_id)
                .add_attribute("relay_chain", chain)
                .add_attribute("recipient", ics20_packet_info.sender)
                .add_attribute("amount", ics20_packet_info.funds.to_string()))
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_ibc_transfer_funds_reply(
    deps: DepsMut,
    _info: MessageInfo,
    env: Env,
    _ctx: Option<AMPPkt>,
    sequence: u64,
    channel_info: ChannelInfo,
    ics20_packet_info: Ics20PacketInfo,
    channel_id: String,
) -> Result<Response, ContractError> {
    let mut ics20_packet_info = ics20_packet_info.clone();
    let chain = &ics20_packet_info
        .recipient
        .get_chain()
        .ok_or(ContractError::InvalidPacket {
            error: Some("Chain not provided in recipient".to_string()),
        })?;

    ensure!(
        !ics20_packet_info.pending,
        ContractError::InvalidPacket {
            error: Some("Packet is pending".to_string()),
        }
    );

    ics20_packet_info.pending = true;
    CHANNEL_TO_EXECUTE_MSG.save(deps.storage, (channel_id, sequence), &ics20_packet_info)?;

    let channel = channel_info
        .direct_channel_id
        .ok_or(ContractError::InvalidPacket {
            error: Some(format!("Direct channel not found for chain {}", chain)),
        })?;

    // TODO: We should send denom info to the counterparty chain with amp message
    let (counterparty_denom, _counterparty_denom_info) = get_counterparty_denom(
        &deps.as_ref(),
        &ics20_packet_info.funds.denom,
        &ics20_packet_info.channel,
    )?;
    #[allow(unused_assignments, unused_mut)]
    let mut adjusted_funds = Coin::new(
        ics20_packet_info.funds.amount.u128(),
        counterparty_denom.clone(),
    );

    // Funds are not correctly hashed when using cw-orchestrator so instead we construct the denom manually
    #[cfg(not(target_arch = "wasm32"))]
    if counterparty_denom.starts_with("ibc/") {
        let hops = path_to_hops(_counterparty_denom_info.path)?;
        // cw-orch doesn't correctly hash the denom so we need to manually construct it
        let adjusted_path = hops
            .iter()
            .map(|hop| hop.channel_id.clone())
            .collect::<Vec<String>>()
            .join("/");

        adjusted_funds = Coin::new(
            ics20_packet_info.funds.amount.u128(),
            format!(
                "ibc/{}/{}",
                adjusted_path, _counterparty_denom_info.base_denom
            ),
        );
    }

    let mut ctx = AMPCtx::new(ics20_packet_info.sender.clone(), env.contract.address, None);

    // Add the orginal sender's username if it exists
    let potential_username = ctx.try_add_origin_username(
        &deps.querier,
        &KERNEL_ADDRESSES.load(deps.storage, VFS_KEY)?,
    );

    // Create a new hop to be appended to the context
    let hop = CrossChainHop::new(
        &channel,
        CURR_CHAIN.load(deps.storage)?,
        chain.to_string(),
        ics20_packet_info.sender.clone(),
        potential_username.clone().map(AndrAddr::from_string),
    );

    // Add the new hop to the context
    ctx.add_hop(hop);

    let kernel_msg = IbcExecuteMsg::SendMessageWithFunds {
        recipient: AndrAddr::from_string(ics20_packet_info.recipient.clone().get_raw_path()),
        message: ics20_packet_info.message,
        funds: adjusted_funds,
        original_sender: ics20_packet_info.sender,
        original_sender_username: potential_username.map(AndrAddr::from_string),
        previous_hops: ctx.previous_hops,
    };
    let msg = IbcMsg::SendPacket {
        channel_id: channel.clone(),
        data: to_json_binary(&kernel_msg)?,
        timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
    };

    Ok(Response::default()
        .add_message(CosmosMsg::Ibc(msg))
        .add_attribute("action", "relay_packet")
        .add_attribute("relay_outcome", "success")
        .add_attribute("relay_sequence", sequence.to_string())
        .add_attribute("relay_channel", ics20_packet_info.channel)
        .add_attribute("relay_chain", chain.to_string())
        .add_attribute("receiving_kernel_address", channel_info.kernel_address))
}

pub fn amp_receive(
    deps: &mut DepsMut,
    info: MessageInfo,
    env: Env,
    packet: AMPPkt,
) -> Result<Response, ContractError> {
    // Only verified ADOs can access this function
    ensure!(
        info.sender == env.contract.address
            || query::verify_address(deps.as_ref(), info.sender.to_string())?.verify_address,
        ContractError::Unauthorized {}
    );

    let mut res = Response::default();
    ensure!(
        !packet.messages.is_empty(),
        ContractError::InvalidPacket {
            error: Some("No messages supplied".to_string())
        }
    );

    let msg = packet.messages.first().unwrap();

    let msg_res = handle(
        deps.branch(),
        info.clone(),
        env.clone(),
        Some(packet.clone()),
        msg.clone(),
    )?;

    res.messages.extend_from_slice(&msg_res.messages);
    res.attributes.extend_from_slice(&msg_res.attributes);
    res.events.extend_from_slice(&msg_res.events);

    let mut new_pkt = AMPPkt::from_ctx(Some(packet.clone()), env.contract.address.to_string());
    new_pkt.ctx.id = Some(generate_or_validate_packet_id(
        deps,
        &env,
        packet.ctx.id.clone(),
    )?);

    for (idx, message) in packet.messages.iter().enumerate() {
        if idx == 0 {
            continue;
        }
        new_pkt = new_pkt.add_message(message.clone());
    }

    if !new_pkt.messages.is_empty() {
        let new_funds = new_pkt
            .messages
            .iter()
            .flat_map(|m| m.funds.clone())
            .collect::<Vec<Coin>>();

        let new_pkt_msg = new_pkt.to_sub_msg(env.contract.address, Some(new_funds), 0)?;
        res.messages.extend_from_slice(&[new_pkt_msg]);
    }

    let message_funds = packet
        .messages
        .iter()
        .flat_map(|m| m.funds.clone())
        .collect::<Vec<Coin>>();
    ensure!(
        has_coins_merged(info.funds.as_slice(), message_funds.as_slice()),
        ContractError::InsufficientFunds {}
    );

    Ok(res.add_attribute("action", "handle_amp_packet"))
}

pub fn handle_cw20(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    ctx: Option<AMPPkt>,
    message: AMPMsg,
) -> Result<Response, ContractError> {
    match message.recipient.get_protocol() {
        Some(_) => Err(ContractError::NotImplemented {
            msg: Some("CW20 over IBC not supported".to_string()),
        }),
        _ => handle_local_cw20(deps, info, env, ctx.map(|ctx| ctx.ctx), message),
    }
}

pub fn amp_receive_cw20(
    deps: &mut DepsMut,
    info: MessageInfo,
    env: Env,
    packet: AMPPkt,
    received_funds: Vec<Coin>,
) -> Result<Response, ContractError> {
    // Only verified ADOs can access this function
    ensure!(
        query::verify_address(deps.as_ref(), info.sender.to_string(),)?.verify_address,
        ContractError::Unauthorized {}
    );

    let mut new_pkt = AMPPkt::from_ctx(Some(packet.clone()), env.contract.address.to_string());

    new_pkt.ctx.id = Some(generate_or_validate_packet_id(
        deps,
        &env,
        packet.ctx.id.clone(),
    )?);

    let mut res = Response::default();
    ensure!(
        !packet.messages.is_empty(),
        ContractError::InvalidPacket {
            error: Some("No messages supplied".to_string())
        }
    );

    for message in packet.messages.iter() {
        let msg_res = handle_cw20(
            deps.branch(),
            info.clone(),
            env.clone(),
            Some(packet.clone()),
            message.clone(),
        )?;
        res.messages.extend_from_slice(&msg_res.messages);
        res.attributes.extend_from_slice(&msg_res.attributes);
        res.events.extend_from_slice(&msg_res.events);
    }

    let message_funds = packet
        .messages
        .iter()
        .flat_map(|m| m.funds.clone())
        .collect::<Vec<Coin>>();
    ensure!(
        has_coins_merged(received_funds.as_slice(), message_funds.as_slice()),
        ContractError::InsufficientFunds {}
    );

    Ok(res.add_attribute("action", "handle_amp_packet"))
}

pub fn upsert_key_address(
    execute_ctx: ExecuteContext,
    key: String,
    value: String,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    ensure!(
        contract.is_contract_owner(execute_ctx.deps.storage, execute_ctx.info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    // Updates to new value
    if KERNEL_ADDRESSES.has(execute_ctx.deps.storage, &key) {
        KERNEL_ADDRESSES.remove(execute_ctx.deps.storage, &key)
    }

    KERNEL_ADDRESSES.save(
        execute_ctx.deps.storage,
        &key,
        &execute_ctx.deps.api.addr_validate(&value)?,
    )?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "upsert_key_address"),
        attr("key", key),
        attr("value", value),
    ]))
}

pub fn create(
    execute_ctx: ExecuteContext,
    ado_type: String,
    msg: Binary,
    owner: Option<AndrAddr>,
    chain: Option<String>,
) -> Result<Response, ContractError> {
    // If chain is provided an owner must be provided
    ensure!(
        chain.is_none() || owner.is_some(),
        ContractError::Unauthorized {}
    );
    if let Some(chain) = chain {
        let cross_chain_components_enabled = ENV_VARIABLES
            .may_load(execute_ctx.deps.storage, "cross_chain_components_enabled")?
            .unwrap_or("false".to_string());
        ensure!(
            cross_chain_components_enabled == "true",
            ContractError::CrossChainComponentsCurrentlyDisabled {}
        );

        let channel_info = if let Some(channel_info) =
            CHAIN_TO_CHANNEL.may_load(execute_ctx.deps.storage, &chain)?
        {
            Ok::<ChannelInfo, ContractError>(channel_info)
        } else {
            return Err(ContractError::InvalidPacket {
                error: Some(format!("Channel not found for chain {chain}")),
            });
        }?;
        let kernel_msg = IbcExecuteMsg::CreateADO {
            instantiation_msg: msg.clone(),
            owner: owner.clone().unwrap(),
            ado_type: ado_type.clone(),
        };
        let ibc_msg = IbcMsg::SendPacket {
            channel_id: channel_info.direct_channel_id.clone().unwrap(),
            data: to_json_binary(&kernel_msg)?,
            timeout: execute_ctx
                .env
                .block
                .time
                .plus_seconds(PACKET_LIFETIME)
                .into(),
        };
        Ok(Response::default()
            .add_message(ibc_msg)
            .add_attributes(vec![
                attr("action", "execute_create"),
                attr("ado_type", ado_type),
                attr("owner", owner.unwrap().to_string()),
                attr("chain", chain),
                attr("receiving_kernel_address", channel_info.kernel_address),
                attr("msg", msg.to_string()),
            ]))
    } else {
        let vfs_addr = KERNEL_ADDRESSES.load(execute_ctx.deps.storage, VFS_KEY)?;
        let adodb_addr = KERNEL_ADDRESSES.load(execute_ctx.deps.storage, ADO_DB_KEY)?;

        let ado_owner = owner.unwrap_or(AndrAddr::from_string(execute_ctx.info.sender.to_string()));
        let owner_addr =
            ado_owner.get_raw_address_from_vfs(&execute_ctx.deps.as_ref(), vfs_addr)?;
        let code_id =
            AOSQuerier::code_id_getter(&execute_ctx.deps.querier, &adodb_addr, &ado_type)?;
        let wasm_msg = WasmMsg::Instantiate {
            admin: Some(owner_addr.to_string()),
            code_id,
            msg,
            funds: vec![],
            label: format!("ADO:{ado_type}"),
        };
        let sub_msg = SubMsg::reply_always(wasm_msg, ReplyId::CreateADO.repr());

        ADO_OWNER.save(execute_ctx.deps.storage, &owner_addr)?;

        Ok(Response::new()
            .add_submessage(sub_msg)
            .add_attribute("action", "execute_create")
            .add_attribute("ado_type", ado_type)
            .add_attribute("owner", ado_owner.to_string()))
    }
}

pub fn internal(ctx: ExecuteContext, msg: InternalMsg) -> Result<Response, ContractError> {
    match msg {
        InternalMsg::RegisterUserCrossChain {
            username,
            address,
            chain,
        } => register_user_cross_chain(ctx, chain, username, address),
    }
}

pub fn register_user_cross_chain(
    execute_ctx: ExecuteContext,
    chain: String,
    username: String,
    address: String,
) -> Result<Response, ContractError> {
    let vfs = KERNEL_ADDRESSES.load(execute_ctx.deps.storage, VFS_KEY)?;
    ensure!(
        execute_ctx.info.sender == vfs,
        ContractError::Unauthorized {}
    );
    let channel_info =
        if let Some(channel_info) = CHAIN_TO_CHANNEL.may_load(execute_ctx.deps.storage, &chain)? {
            Ok::<ChannelInfo, ContractError>(channel_info)
        } else {
            return Err(ContractError::InvalidPacket {
                error: Some(format!("Channel not found for chain {chain}")),
            });
        }?;
    let kernel_msg = IbcExecuteMsg::RegisterUsername {
        username: username.clone(),
        address: address.clone(),
    };
    let channel_id = if let Some(direct_channel_id) = channel_info.direct_channel_id {
        Ok::<String, ContractError>(direct_channel_id)
    } else {
        return Err(ContractError::InvalidPacket {
            error: Some(format!("Channel not found for chain {chain}")),
        });
    }?;
    let ibc_msg = IbcMsg::SendPacket {
        channel_id,
        data: to_json_binary(&kernel_msg)?,
        timeout: execute_ctx
            .env
            .block
            .time
            .plus_seconds(PACKET_LIFETIME)
            .into(),
    };

    Ok(Response::default()
        .add_attributes(vec![
            attr("action", "register_user_cross_chain"),
            attr("username", username),
            attr("address", address),
            attr("chain", chain),
            attr("receiving_kernel_address", channel_info.kernel_address),
        ])
        .add_message(ibc_msg))
}

pub fn assign_channels(
    execute_ctx: ExecuteContext,
    ics20_channel_id: Option<String>,
    direct_channel_id: Option<String>,
    chain: String,
    kernel_address: String,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    ensure!(
        contract.is_contract_owner(execute_ctx.deps.storage, execute_ctx.info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let mut channel_info = CHAIN_TO_CHANNEL
        .load(execute_ctx.deps.storage, &chain)
        .unwrap_or_default();
    channel_info.kernel_address = kernel_address;
    if let Some(channel) = direct_channel_id {
        // Remove old direct channel to chain if it exists
        if let Some(direct_channel_id) = channel_info.direct_channel_id {
            CHANNEL_TO_CHAIN.remove(execute_ctx.deps.storage, &direct_channel_id);
        }
        CHANNEL_TO_CHAIN.save(execute_ctx.deps.storage, &channel, &chain)?;
        channel_info.direct_channel_id = Some(channel);
    }
    if let Some(channel) = ics20_channel_id {
        // Remove old ics20 channel to chain if it exists
        if let Some(ics20_channel_id) = channel_info.ics20_channel_id {
            CHANNEL_TO_CHAIN.remove(execute_ctx.deps.storage, &ics20_channel_id);
        }
        CHANNEL_TO_CHAIN.save(execute_ctx.deps.storage, &channel, &chain)?;
        channel_info.ics20_channel_id = Some(channel);
    }
    CHAIN_TO_CHANNEL.save(execute_ctx.deps.storage, &chain, &channel_info)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "assign_channel"),
        attr(
            "ics20_channel_id",
            channel_info.ics20_channel_id.unwrap_or("None".to_string()),
        ),
        attr(
            "direct_channel_id",
            channel_info.direct_channel_id.unwrap_or("None".to_string()),
        ),
        attr("chain", chain),
        attr("kernel_address", channel_info.kernel_address),
    ]))
}

pub fn recover(execute_ctx: ExecuteContext) -> Result<Response, ContractError> {
    let recoveries = IBC_FUND_RECOVERY
        .load(execute_ctx.deps.storage, &execute_ctx.info.sender)
        .unwrap_or_default();
    IBC_FUND_RECOVERY.remove(execute_ctx.deps.storage, &execute_ctx.info.sender);
    ensure!(
        !recoveries.is_empty(),
        ContractError::Std(StdError::generic_err("No recoveries found"))
    );

    let bank_msg = BankMsg::Send {
        to_address: execute_ctx.info.sender.to_string(),
        amount: recoveries,
    };
    let sub_msg = SubMsg::reply_always(bank_msg, ReplyId::Recovery.repr());

    Ok(Response::default()
        .add_attribute("action", "recover")
        .add_submessage(sub_msg))
}

pub fn update_chain_name(
    execute_ctx: ExecuteContext,
    chain_name: String,
) -> Result<Response, ContractError> {
    // Only owner can update CURR_CHAIN
    let contract = ADOContract::default();
    ensure!(
        contract.is_contract_owner(execute_ctx.deps.storage, execute_ctx.info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    // Update CURR_CHAIN
    CURR_CHAIN.save(execute_ctx.deps.storage, &chain_name)?;

    Ok(Response::default()
        .add_attribute("action", "update_chain_name")
        .add_attribute("sender", execute_ctx.info.sender.as_str())
        .add_attribute("chain_name", chain_name))
}

pub fn set_env(
    execute_ctx: ExecuteContext,
    variable: String,
    value: String,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    ensure!(
        contract.is_contract_owner(execute_ctx.deps.storage, execute_ctx.info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    ensure!(
        !variable.is_empty(),
        ContractError::InvalidEnvironmentVariable {
            msg: "Environment variable name cannot be empty".to_string()
        }
    );

    ensure!(
        variable
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_'),
        ContractError::InvalidEnvironmentVariable {
            msg:
                "Environment variable name can only contain alphanumeric characters and underscores"
                    .to_string()
        }
    );

    ensure!(
        variable.len() <= 100,
        ContractError::InvalidEnvironmentVariable {
            msg: "Environment variable name length exceeds the maximum allowed length of 100 characters".to_string()
        }
    );

    ensure!(
        !value.is_empty(),
        ContractError::InvalidEnvironmentVariable {
            msg: "Environment variable value cannot be empty".to_string()
        }
    );

    ensure!(
        value.len() <= 100,
        ContractError::InvalidEnvironmentVariable {
            msg: "Environment variable value length exceeds the maximum allowed length of 100 characters".to_string()
        }
    );

    ENV_VARIABLES.save(
        execute_ctx.deps.storage,
        &variable.to_ascii_uppercase(),
        &value,
    )?;
    Ok(Response::default()
        .add_attribute("action", "set_env")
        .add_attribute("variable", variable)
        .add_attribute("value", value))
}

pub fn unset_env(execute_ctx: ExecuteContext, variable: String) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    ensure!(
        contract.is_contract_owner(execute_ctx.deps.storage, execute_ctx.info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    ensure!(
        ENV_VARIABLES
            .may_load(execute_ctx.deps.storage, &variable.to_ascii_uppercase())?
            .is_some(),
        ContractError::EnvironmentVariableNotFound { variable }
    );
    ENV_VARIABLES.remove(execute_ctx.deps.storage, &variable.to_ascii_uppercase());
    Ok(Response::default()
        .add_attribute("action", "unset_env")
        .add_attribute("variable", variable))
}

/// Generates or validates a packet ID using chain ID, block height and transaction index
fn generate_or_validate_packet_id(
    deps: &mut DepsMut,
    env: &Env,
    existing_id: Option<String>,
) -> Result<String, ContractError> {
    let tx_index = TX_INDEX.may_load(deps.storage)?.unwrap_or_default();

    TX_INDEX.save(deps.storage, &tx_index.checked_add(Uint128::one())?)?;

    match existing_id {
        // Generate unique ID if the packet doesn't already have one
        Some(id) => validate_id(&id, &env.block.chain_id, env.block.height, tx_index),
        // Not using "-" as a separator since chain id can contain it
        None => Ok(format!(
            "{}.{}.{}",
            env.block.chain_id, env.block.height, tx_index
        )),
    }
}

/// Validates the existing ID of a packet
pub fn validate_id(
    id: &str,
    current_chain_id: &str,
    current_block_height: u64,
    current_index: Uint128,
) -> Result<String, ContractError> {
    // Split the ID into chain_id, block_height, and index parts
    let parts: Vec<&str> = id.split('.').collect();
    if parts.len() != 3 {
        return Err(ContractError::InvalidPacket {
            error: Some(
                "Invalid packet ID format. Expected: chain_id.block_height.index".to_string(),
            ),
        });
    }

    let [chain_id, block_height_str, index_str] = [parts[0], parts[1], parts[2]];

    // Validate chain_id
    if chain_id.is_empty() {
        return Err(ContractError::InvalidPacket {
            error: Some("Chain ID cannot be empty".to_string()),
        });
    }

    // Parse and validate block height and index
    let block_height =
        block_height_str
            .parse::<u64>()
            .map_err(|_| ContractError::InvalidPacket {
                error: Some("Invalid block height format".to_string()),
            })?;

    let index = index_str
        .parse::<Uint128>()
        .map_err(|_| ContractError::InvalidPacket {
            error: Some("Invalid transaction index format".to_string()),
        })?;

    //TODO discuss validation for cross chain packets
    if chain_id == current_chain_id
        && (block_height != current_block_height || index != current_index)
    {
        return Err(ContractError::InvalidPacket {
            error: Some(
                "Block height or transaction index does not match the current values".to_string(),
            ),
        });
    }

    Ok(id.to_string())
}

/// Handles an Inter-Blockchain Communication (IBC) message.
/// Handles a given AMP message and returns a response
/// This function routes messages to different chains based on the protocol specified in the recipient's
/// VFS path (ibc://<chain>/path). It performs the following checks:
/// 1. Verifies that a destination chain is specified
/// 2. Confirms a valid channel configuration exists for that chain
/// 3. Routes to the appropriate handler based on whether funds are included
///
/// # Parameters
/// * `deps` - Mutable access to contract storage and APIs
/// * `info` - Information about the message sender
/// * `env` - Environment information (contract address, block height, etc.)
/// * `ctx` - Optional context from previous message handling
/// * `message` - The AMP message to be sent across chains
///
/// # Returns
/// * `Ok(Response)` - Response with the prepared IBC message
/// * `Err(ContractError)` - Error if chain validation fails or appropriate channel not found
fn handle_ibc(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    ctx: Option<AMPPkt>,
    message: AMPMsg,
) -> Result<Response, ContractError> {
    // check if chain is provided
    let chain = message
        .recipient
        .get_chain()
        .ok_or(ContractError::InvalidPacket {
            error: Some("Chain not provided".to_string()),
        })?;

    //check if channel is present
    let channel_info =
        CHAIN_TO_CHANNEL
            .may_load(deps.storage, chain)?
            .ok_or(ContractError::InvalidPacket {
                error: Some(format!("Channel not found for chain {chain}")),
            })?;

    //check if funds are present
    if message.funds.is_empty() {
        handle_ibc_direct(deps, info, env, ctx, message, channel_info)
    } else {
        handle_ibc_transfer_funds(deps, info, env, ctx, message, channel_info)
    }
}

/// Builds an AMP packet for cross-chain communication via IBC
/// This function prepares a message to be sent across chains by:
/// 1. Setting up the message content
/// 2. Creating or updating the message context with routing information
/// 3. Adding username data and hop information for cross-chain tracking
fn build_ibc_packet(
    deps: &DepsMut,
    info: &MessageInfo,
    env: &Env,
    channel: &str,
    existing_packet: Option<AMPPkt>,
    current_chain: String,
    amp_message: AMPMsg,
) -> Result<AMPPkt, ContractError> {
    // Get VFS address for username lookups
    let vfs_address =
        KERNEL_ADDRESSES
            .may_load(deps.storage, VFS_KEY)?
            .ok_or(ContractError::Std(StdError::not_found(
                "VFSAddressNotFound",
            )))?;

    // Prepare message with appropriate content and config
    let amp_msg = AMPMsg::new(
        amp_message.recipient.get_raw_path(),
        amp_message.message.clone(),
        None,
    )
    .with_config(amp_message.config.clone());

    // Set up the context - either create new or use existing
    let mut ctx = if let Some(mut packet) = existing_packet {
        // Update existing packet's recipient
        packet.messages[0].recipient =
            AndrAddr::from_string(amp_message.recipient.get_raw_path().to_string());
        packet.ctx
    } else {
        // Create new context with origin information
        AMPCtx::new(info.sender.clone(), env.contract.address.clone(), None)
    };

    // Add username information if available
    let username = ctx.try_add_origin_username(&deps.querier, &vfs_address);

    // Extract the destination chain from the recipient with proper error handling
    let destination_chain =
        amp_message
            .recipient
            .get_chain()
            .ok_or(ContractError::InvalidPacket {
                error: Some("Destination chain not found in recipient".to_string()),
            })?;

    // Create and add routing information using the constructor directly
    let hop = CrossChainHop::new(
        channel,
        current_chain,
        destination_chain.to_string(),
        ctx.get_origin(),
        username.map(AndrAddr::from_string),
    );
    ctx.add_hop(hop);

    // Assemble the final packet
    Ok(AMPPkt::new_with_ctx(ctx, vec![amp_msg]))
}

/// Handles direct IBC message sending without funds transfer.
///
/// This function processes messages that need to be sent to other chains but don't
/// involve token transfers. It performs the following steps:
/// 1. Validates that the message is not empty
/// 2. Extracts the destination chain from the recipient
/// 3. Retrieves the direct channel ID for the destination chain
/// 4. Builds an IBC packet with routing information
/// 5. Creates and sends the IBC message via the appropriate channel
///
/// # Parameters
/// * `deps` - Mutable access to contract storage and APIs
/// * `info` - Information about the message sender
/// * `env` - Environment information (contract address, block height, etc.)
/// * `pkt` - Optional existing packet context
/// * `amp_message` - The AMP message to be sent
/// * `channel_info` - Information about the channel to the destination chain
///
/// # Returns
/// * `Ok(Response)` - Response with the IBC message
/// * `Err(ContractError)` - Error if validation fails or required channel not found
pub(crate) fn handle_ibc_direct(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    pkt: Option<AMPPkt>,
    amp_message: AMPMsg,
    channel_info: ChannelInfo,
) -> Result<Response, ContractError> {
    // Validate that the message is not empty
    ensure!(
        !amp_message.message.is_empty(),
        ContractError::InvalidPacket {
            error: Some("Cannot send an empty message without funds via IBC".to_string())
        }
    );

    // Extract the destination chain from the recipient with proper error handling
    let destination_chain =
        amp_message
            .recipient
            .get_chain()
            .ok_or(ContractError::InvalidPacket {
                error: Some("Destination chain not found in recipient".to_string()),
            })?;

    // Retrieve the direct channel ID for the destination chain
    let channel = channel_info
        .direct_channel_id
        .ok_or(ContractError::InvalidPacket {
            error: Some(format!(
                "Direct channel not found for chain {}",
                &destination_chain
            )),
        })?;

    // Build IBC context
    let current_chain = CURR_CHAIN.load(deps.storage)?;

    let amp_pkt = build_ibc_packet(
        &deps,
        &info,
        &env,
        &channel,
        pkt,
        current_chain,
        amp_message.clone(),
    )?;

    // Create and send IBC message
    let kernel_msg = IbcExecuteMsg::SendMessage {
        amp_packet: amp_pkt,
    };

    let msg = IbcMsg::SendPacket {
        channel_id: channel.clone(),
        data: to_json_binary(&kernel_msg)?,
        timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
    };

    Ok(Response::default()
        .add_attribute("method", "execute_send_message")
        .add_attribute("channel", channel)
        .add_attribute("receiving_kernel_address", channel_info.kernel_address)
        .add_attribute("chain", destination_chain)
        .add_message(msg))
}

/// Handles IBC messages that include funds transfer.
///
/// This function processes cross-chain messages that involve token transfers.
/// It performs the following steps:
/// 1. Extracts the destination chain and validates it exists
/// 2. Finds the appropriate ICS20 channel for funds transfer
/// 3. Validates the funds (must be exactly one coin)
/// 4. Creates an IBC transfer message
/// 5. Stores the message details for later processing in a reply handler
///
/// The funds are first transferred via ICS20, and upon successful transfer,
/// the associated message is sent in a follow-up transaction.
///
/// # Parameters
/// * `deps` - Mutable access to contract storage and APIs
/// * `info` - Information about the message sender
/// * `env` - Environment information (contract address, block height, etc.)
/// * `_ctx` - Optional context from previous message handling
/// * `message` - The AMP message containing funds to be sent
/// * `channel_info` - Information about the channels to the destination chain
///
/// # Returns
/// * `Ok(Response)` - Response with the IBC transfer submessage
/// * `Err(ContractError)` - Error if validation fails or required channel not found
pub(crate) fn handle_ibc_transfer_funds(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    _ctx: Option<AMPPkt>,
    message: AMPMsg,
    channel_info: ChannelInfo,
) -> Result<Response, ContractError> {
    let AMPMsg {
        recipient,
        message,
        funds,
        ..
    } = message;

    //check if chain is provided
    let chain = recipient.get_chain().ok_or(ContractError::InvalidPacket {
        error: Some("Chain not provided in recipient".to_string()),
    })?;

    // We know the channel_info exists, but we need the specific ics20_channel_id
    let channel = channel_info
        .ics20_channel_id
        .ok_or(ContractError::InvalidPacket {
            error: Some(format!("ICS20 channel not found for chain {chain}")),
        })?;

    //check if funds are present
    ensure!(
        funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Number of funds should be exactly one".to_string()
        }
    );
    let coin = funds
        .first()
        .ok_or(ContractError::InvalidPacket {
            error: Some("Transfer funds must contain funds in the AMPMsg".to_string()),
        })?
        .clone();

    let msg = IbcMsg::Transfer {
        channel_id: channel.clone(),
        to_address: channel_info.kernel_address.clone(),
        amount: coin.clone(),
        timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
    };
    let mut resp = Response::default();

    // Save execute msg, to be loaded in the reply
    PENDING_MSG_AND_FUNDS.save(
        deps.storage,
        &Ics20PacketInfo {
            sender: info.sender.into_string(),
            recipient: recipient.clone(),
            message: message.clone(),
            funds: coin,
            channel: channel.clone(),
            pending: false,
        },
    )?;
    resp = resp.add_submessage(SubMsg {
        id: ReplyId::IBCTransfer.repr(),
        msg: CosmosMsg::Ibc(msg),
        gas_limit: None,
        reply_on: cosmwasm_std::ReplyOn::Always,
    });

    Ok(resp
        .add_attribute("method", "execute_transfer_funds")
        .add_attribute("channel", channel)
        .add_attribute("receiving_kernel_address", channel_info.kernel_address)
        .add_attribute("chain", chain))
}
