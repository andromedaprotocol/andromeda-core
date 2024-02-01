use andromeda_std::ado_contract::ADOContract;
use andromeda_std::amp::addresses::AndrAddr;
use andromeda_std::amp::messages::{AMPMsg, AMPPkt, IBCConfig};
use andromeda_std::amp::{ADO_DB_KEY, VFS_KEY};

use andromeda_std::common::context::ExecuteContext;
use andromeda_std::error::ContractError;
use andromeda_std::os::aos_querier::AOSQuerier;
use andromeda_std::os::kernel::{ChannelInfo, IbcExecuteMsg, InternalMsg};

use andromeda_std::os::vfs::vfs_resolve_symlink;
use cosmwasm_std::{
    attr, ensure, to_binary, Addr, BankMsg, Binary, CosmosMsg, DepsMut, Env, IbcMsg, MessageInfo,
    Response, StdError, SubMsg, WasmMsg,
};

use crate::ibc::{generate_transfer_message, PACKET_LIFETIME};
use crate::state::{
    IBCHooksPacketSendState, ADO_OWNER, CHAIN_TO_CHANNEL, CHANNEL_TO_CHAIN, IBC_FUND_RECOVERY,
    KERNEL_ADDRESSES, OUTGOING_IBC_HOOKS_PACKETS,
};
use crate::{query, reply::ReplyId};

pub fn send(ctx: ExecuteContext, message: AMPMsg) -> Result<Response, ContractError> {
    let res = MsgHandler(message).handle(ctx.deps, ctx.info, ctx.env, ctx.amp_ctx, 0)?;

    Ok(res)
}

pub fn amp_receive(
    deps: &mut DepsMut,
    info: MessageInfo,
    env: Env,
    packet: AMPPkt,
) -> Result<Response, ContractError> {
    ensure!(
        query::verify_address(deps.as_ref(), info.sender.to_string(),)?
            || packet.ctx.get_origin() == info.sender,
        ContractError::Unauthorized {}
    );
    ensure!(
        packet.ctx.id == 0,
        ContractError::InvalidPacket {
            error: Some("Packet ID cannot be provided from outside the Kernel".into())
        }
    );

    let mut res = Response::default();
    ensure!(
        !packet.messages.is_empty(),
        ContractError::InvalidPacket {
            error: Some("No messages supplied".to_string())
        }
    );
    for (idx, message) in packet.messages.iter().enumerate() {
        let mut handler = MsgHandler::new(message.clone());
        res = handler.handle(
            deps.branch(),
            info.clone(),
            env.clone(),
            Some(packet.clone()),
            idx as u64,
        )?;
    }
    Ok(res.add_attribute("action", "handle_amp_packet"))
}

pub fn upsert_key_address(
    execute_env: ExecuteContext,
    key: String,
    value: String,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    ensure!(
        contract.is_contract_owner(execute_env.deps.storage, execute_env.info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    // Updates to new value
    if KERNEL_ADDRESSES.has(execute_env.deps.storage, &key) {
        KERNEL_ADDRESSES.remove(execute_env.deps.storage, &key)
    }

    KERNEL_ADDRESSES.save(
        execute_env.deps.storage,
        &key,
        &execute_env.deps.api.addr_validate(&value)?,
    )?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "upsert_key_address"),
        attr("key", key),
        attr("value", value),
    ]))
}

pub fn create(
    execute_env: ExecuteContext,
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
        let channel_info = if let Some(channel_info) =
            CHAIN_TO_CHANNEL.may_load(execute_env.deps.storage, &chain)?
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
            data: to_binary(&kernel_msg)?,
            timeout: execute_env
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
        let vfs_addr = KERNEL_ADDRESSES.load(execute_env.deps.storage, VFS_KEY)?;
        let adodb_addr = KERNEL_ADDRESSES.load(execute_env.deps.storage, ADO_DB_KEY)?;

        let ado_owner = owner.unwrap_or(AndrAddr::from_string(execute_env.info.sender.to_string()));
        let owner_addr =
            ado_owner.get_raw_address_from_vfs(&execute_env.deps.as_ref(), vfs_addr)?;
        let code_id =
            AOSQuerier::code_id_getter(&execute_env.deps.querier, &adodb_addr, &ado_type)?;
        let wasm_msg = WasmMsg::Instantiate {
            admin: Some(owner_addr.to_string()),
            code_id,
            msg,
            funds: vec![],
            label: format!("ADO:{ado_type}"),
        };
        let sub_msg = SubMsg::reply_always(wasm_msg, ReplyId::CreateADO.repr());

        // TODO: Is this check necessary?
        // ensure!(
        //     !ADO_OWNER.exists(execute_env.deps.storage),
        //     ContractError::Unauthorized {}
        // );

        ADO_OWNER.save(execute_env.deps.storage, &owner_addr)?;

        Ok(Response::new()
            .add_submessage(sub_msg)
            .add_attribute("action", "execute_create")
            .add_attribute("ado_type", ado_type)
            .add_attribute("owner", ado_owner.to_string()))
    }
}

pub fn internal(env: ExecuteContext, msg: InternalMsg) -> Result<Response, ContractError> {
    match msg {
        InternalMsg::RegisterUserCrossChain {
            username,
            address,
            chain,
        } => register_user_cross_chain(env, chain, username, address),
    }
}

pub fn register_user_cross_chain(
    execute_env: ExecuteContext,
    chain: String,
    username: String,
    address: String,
) -> Result<Response, ContractError> {
    let vfs = KERNEL_ADDRESSES.load(execute_env.deps.storage, VFS_KEY)?;
    ensure!(
        execute_env.info.sender == vfs,
        ContractError::Unauthorized {}
    );
    let channel_info =
        if let Some(channel_info) = CHAIN_TO_CHANNEL.may_load(execute_env.deps.storage, &chain)? {
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
    let ibc_msg = IbcMsg::SendPacket {
        channel_id: channel_info.direct_channel_id.clone().unwrap(),
        data: to_binary(&kernel_msg)?,
        timeout: execute_env
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
    execute_env: ExecuteContext,
    ics20_channel_id: Option<String>,
    direct_channel_id: Option<String>,
    chain: String,
    kernel_address: String,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    ensure!(
        contract.is_contract_owner(execute_env.deps.storage, execute_env.info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let mut channel_info = CHAIN_TO_CHANNEL.load(execute_env.deps.storage, &chain)?;
    channel_info.kernel_address = kernel_address;
    if let Some(channel) = direct_channel_id {
        CHANNEL_TO_CHAIN.save(execute_env.deps.storage, &channel, &chain)?;
        channel_info.direct_channel_id = Some(channel);
    }
    if let Some(channel) = ics20_channel_id {
        CHANNEL_TO_CHAIN.save(execute_env.deps.storage, &channel, &chain)?;
        channel_info.ics20_channel_id = Some(channel);
    }
    CHAIN_TO_CHANNEL.save(execute_env.deps.storage, &chain, &channel_info)?;

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

pub fn recover(execute_env: ExecuteContext) -> Result<Response, ContractError> {
    let recoveries = IBC_FUND_RECOVERY
        .load(execute_env.deps.storage, &execute_env.info.sender)
        .unwrap_or_default();
    IBC_FUND_RECOVERY.remove(execute_env.deps.storage, &execute_env.info.sender);
    ensure!(
        !recoveries.is_empty(),
        ContractError::Std(StdError::generic_err("No recoveries found"))
    );

    let bank_msg = BankMsg::Send {
        to_address: execute_env.info.sender.to_string(),
        amount: recoveries,
    };
    let sub_msg = SubMsg::reply_always(bank_msg, ReplyId::Recovery.repr());

    Ok(Response::default()
        .add_attribute("action", "recover")
        .add_submessage(sub_msg))
}

/// Handles a given AMP message and returns a response
///
/// Separated due to common functionality across multiple messages
#[derive(Clone)]
struct MsgHandler(AMPMsg);

impl MsgHandler {
    pub fn new(msg: AMPMsg) -> Self {
        Self(msg)
    }

    fn message(&self) -> &AMPMsg {
        &self.0
    }

    fn update_recipient(&mut self, recipient: AndrAddr) -> Self {
        self.0.recipient = recipient;
        self.clone()
    }

    #[inline]
    pub fn handle(
        &mut self,
        deps: DepsMut,
        info: MessageInfo,
        env: Env,
        ctx: Option<AMPPkt>,
        sequence: u64,
    ) -> Result<Response, ContractError> {
        let resolved_recipient = if self.message().recipient.is_vfs_path() {
            let vfs_address = KERNEL_ADDRESSES.load(deps.storage, VFS_KEY)?;
            vfs_resolve_symlink(
                self.message().recipient.clone(),
                vfs_address.to_string(),
                &deps.querier,
            )?
        } else {
            self.message().recipient.clone()
        };
        self.update_recipient(resolved_recipient);
        let protocol = self.message().recipient.get_protocol();

        match protocol {
            Some("ibc") => self.handle_ibc(deps, info, env, ctx, sequence),
            _ => self.handle_local(deps, info, env, ctx, sequence),
        }
    }

    /**
    Handles a local AMP Message, that is a message that has no defined protocol in its recipient VFS path. There are two different situations for a local message that are defined by the binary message provided.
    Situation 1 is that the message provided is empty or `Binary::default` in which case the message must be a `BankMsg::Send` message and the funds must be provided.
    Situation 2 is that the message has a provided binary and must be a `WasmMsg::Execute` message.

    In both situations the sender can define the funds that are being attached to the message.
    */
    fn handle_local(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        _env: Env,
        ctx: Option<AMPPkt>,
        sequence: u64,
    ) -> Result<Response, ContractError> {
        let mut res = Response::default();
        let AMPMsg {
            message,
            recipient,
            funds,
            ..
        } = self.message();
        let recipient_addr = recipient.get_raw_address(&deps.as_ref())?;

        // A default message is a bank message
        if Binary::default() == message.clone() {
            ensure!(
                !funds.is_empty(),
                ContractError::InvalidPacket {
                    error: Some("No message or funds supplied".to_string())
                }
            );

            let sub_msg = BankMsg::Send {
                to_address: recipient_addr.to_string(),
                amount: funds.clone(),
            };

            let mut attrs = vec![];
            for (idx, fund) in funds.iter().enumerate() {
                attrs.push(attr(format!("funds:{sequence}:{idx}"), fund.to_string()));
            }
            attrs.push(attr(format!("recipient:{sequence}"), recipient_addr));
            res = res
                .add_submessage(SubMsg::reply_on_error(
                    CosmosMsg::Bank(sub_msg),
                    ReplyId::AMPMsg.repr(),
                ))
                .add_attributes(attrs);
        } else {
            let origin = if let Some(amp_ctx) = ctx {
                amp_ctx.ctx.get_origin()
            } else {
                info.sender.to_string()
            };
            let previous_sender = info.sender.to_string();

            let amp_msg = AMPMsg::new(recipient_addr.clone(), message.clone(), Some(funds.clone()));

            let new_packet = AMPPkt::new(origin, previous_sender, vec![amp_msg]);

            //TODO: Check funds are sent with message
            let sub_msg = new_packet.to_sub_msg(
                recipient_addr.clone(),
                Some(funds.clone()),
                ReplyId::AMPMsg.repr(),
            )?;
            res = res
                .add_submessage(sub_msg)
                .add_attributes(vec![attr(format!("recipient:{sequence}"), recipient_addr)]);
        }
        Ok(res)
    }

    /**
    Handles an IBC AMP Message. An IBC AMP Message is defined by adding the `ibc://<chain>` protocol definition to the start of the VFS path.
    The `chain` is the chain ID of the destination chain and an appropriate channel must be present for the given chain.

    The VFS path has its protocol stripped and the message is passed via ibc-hooks to the kernel on the receiving chain. The kernel on the receiving chain will receive the message as if it was sent from the local chain and will act accordingly.
    */
    fn handle_ibc(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        env: Env,
        ctx: Option<AMPPkt>,
        sequence: u64,
    ) -> Result<Response, ContractError> {
        if let Some(chain) = self.message().recipient.get_chain() {
            let channel_info =
                if let Some(channel_info) = CHAIN_TO_CHANNEL.may_load(deps.storage, chain)? {
                    Ok::<ChannelInfo, ContractError>(channel_info)
                } else {
                    return Err(ContractError::InvalidPacket {
                        error: Some(format!("Channel not found for chain {chain}")),
                    });
                }?;
            if !self.message().funds.is_empty() {
                self.handle_ibc_hooks(deps, info, env, ctx, sequence, channel_info)
            } else {
                self.handle_ibc_direct(deps, info, env, ctx, sequence, channel_info)
            }
        } else {
            Err(ContractError::InvalidPacket {
                error: Some("Chain not provided".to_string()),
            })
        }
    }

    fn handle_ibc_direct(
        &self,
        _deps: DepsMut,
        _info: MessageInfo,
        env: Env,
        _ctx: Option<AMPPkt>,
        sequence: u64,
        channel_info: ChannelInfo,
    ) -> Result<Response, ContractError> {
        let AMPMsg {
            recipient, message, ..
        } = self.message();
        ensure!(
            Binary::default().eq(message),
            ContractError::InvalidPacket {
                error: Some("Cannot send an empty message without funds via IBC".to_string())
            }
        );
        let chain = recipient.get_chain().unwrap();
        let channel = if let Some(direct_channel) = channel_info.direct_channel_id {
            Ok::<String, ContractError>(direct_channel)
        } else {
            return Err(ContractError::InvalidPacket {
                error: Some(format!("Channel not found for chain {chain}")),
            });
        }?;

        let kernel_msg = IbcExecuteMsg::SendMessage {
            recipient: AndrAddr::from_string(recipient.get_raw_path()),
            message: message.clone(),
        };
        let msg = IbcMsg::SendPacket {
            channel_id: channel.clone(),
            data: to_binary(&kernel_msg)?,
            timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
        };

        Ok(Response::default()
            .add_attribute(format!("method:{sequence}"), "execute_send_message")
            .add_attribute(format!("channel:{sequence}"), channel)
            .add_attribute("receiving_kernel_address:{}", channel_info.kernel_address)
            .add_attribute("chain:{}", chain)
            .add_message(msg))
    }

    fn handle_ibc_hooks(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        env: Env,
        ctx: Option<AMPPkt>,
        sequence: u64,
        channel_info: ChannelInfo,
    ) -> Result<Response, ContractError> {
        let AMPMsg {
            recipient,
            message,
            funds,
            config,
            ..
        } = self.message();
        let chain = recipient.get_chain().unwrap();
        let channel = if let Some(ics20_channel) = channel_info.ics20_channel_id {
            Ok::<String, ContractError>(ics20_channel)
        } else {
            return Err(ContractError::InvalidPacket {
                error: Some(format!("Channel not found for chain {chain}")),
            });
        }?;
        let msg_funds = &funds[0].clone();
        let recovery_addr = if let Some(IBCConfig {
            recovery_addr: Some(recovery_addr),
        }) = config.ibc_config.clone()
        {
            let addr = recovery_addr.get_raw_address(&deps.as_ref())?;
            Ok::<Addr, ContractError>(addr)
        } else if let Some(AMPPkt { ctx, .. }) = ctx {
            Ok::<Addr, ContractError>(deps.api.addr_validate(&ctx.get_origin())?)
        } else {
            Ok::<Addr, ContractError>(info.sender)
        }?;
        let outgoing_state = IBCHooksPacketSendState {
            channel_id: channel.clone(),
            amount: msg_funds.clone(),
            recovery_addr,
        };

        let mut outgoing_packets = OUTGOING_IBC_HOOKS_PACKETS
            .load(deps.storage)
            .unwrap_or_default();
        outgoing_packets.push(outgoing_state);
        OUTGOING_IBC_HOOKS_PACKETS.save(deps.storage, &outgoing_packets)?;

        let msg = generate_transfer_message(
            &deps.as_ref(),
            recipient.clone(),
            message.clone(),
            msg_funds.clone(),
            channel.clone(),
            env.contract.address.to_string(),
            channel_info.kernel_address.clone(),
            env.block.time,
        )?;
        Ok(Response::default()
            .add_submessage(SubMsg::reply_always(
                msg,
                ReplyId::IBCHooksPacketSend.repr(),
            ))
            .add_attribute(format!("method:{sequence}"), "execute_send_message")
            .add_attribute(format!("channel:{sequence}"), channel)
            .add_attribute(
                format!("receiving_kernel_address:{sequence}"),
                channel_info.kernel_address,
            )
            .add_attribute(format!("chain:{sequence}"), chain))
    }
}
