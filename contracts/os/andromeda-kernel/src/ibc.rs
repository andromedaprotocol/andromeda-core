use crate::ack::{make_ack_fail, make_ack_success};
use crate::execute;
use crate::state::{CHAIN_TO_CHANNEL, CHANNEL_TO_CHAIN, KERNEL_ADDRESSES, REFUND_DATA};
use andromeda_std::amp::messages::{AMPCtx, AMPPkt};
use andromeda_std::amp::{IBC_REGISTRY_KEY, VFS_KEY};
use andromeda_std::common::context::ExecuteContext;
use andromeda_std::common::reply::ReplyId;
use andromeda_std::error::{ContractError, Never};
use andromeda_std::os::aos_querier::AOSQuerier;
use andromeda_std::os::ibc_registry::DenomInfo;
use andromeda_std::os::kernel::RefundData;
use andromeda_std::os::IBC_VERSION;
use andromeda_std::{
    amp::{messages::AMPMsg, AndrAddr},
    os::{kernel::IbcExecuteMsg, vfs::ExecuteMsg as VFSExecuteMsg},
};
use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, from_json, to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env,
    Ibc3ChannelOpenResponse, IbcBasicResponse, IbcChannel, IbcChannelCloseMsg,
    IbcChannelConnectMsg, IbcChannelOpenMsg, IbcOrder, IbcPacketAckMsg, IbcPacketReceiveMsg,
    IbcPacketTimeoutMsg, IbcReceiveResponse, MessageInfo, SubMsg, WasmMsg,
};

pub const PACKET_LIFETIME: u64 = 604_800u64;

#[cw_serde]
pub enum IBCLifecycleComplete {
    #[serde(rename = "ibc_ack")]
    IBCAck {
        channel: String,
        sequence: u64,
        ack: String,
        success: bool,
    },
    #[serde(rename = "ibc_timeout")]
    IBCTimeout { channel: String, sequence: u64 },
}

#[cw_serde]
pub enum SudoMsg {
    #[serde(rename = "ibc_lifecycle_complete")]
    IBCLifecycleComplete(IBCLifecycleComplete),
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // As with ack above, nothing to do here. If we cared about
    // keeping track of state between the two chains then we'd want to
    // respond to this likely as it means that the packet in question
    // isn't going anywhere.
    Ok(IbcBasicResponse::new().add_attribute("method", "ibc_packet_timeout"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<Option<Ibc3ChannelOpenResponse>, ContractError> {
    validate_order_and_version(msg.channel(), msg.counterparty_version())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_connect(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> Result<IbcBasicResponse, ContractError> {
    validate_order_and_version(msg.channel(), msg.counterparty_version())?;

    let channel = msg.channel().endpoint.channel_id.clone();

    Ok(IbcBasicResponse::new()
        .add_attribute("method", "ibc_channel_connect")
        .add_attribute("channel_id", channel))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> Result<IbcBasicResponse, ContractError> {
    let channel = msg.channel().endpoint.channel_id.clone();
    // Reset the state for the channel.
    Ok(IbcBasicResponse::new()
        .add_attribute("method", "ibc_channel_close")
        .add_attribute("channel", channel))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_receive(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, Never> {
    // Regardless of if our processing of this packet works we need to
    // commit an ACK to the chain. As such, we wrap all handling logic
    // in a seprate function and on error write out an error ack.
    match do_ibc_packet_receive(deps, env, msg) {
        Ok(response) => Ok(response),
        Err(error) => Ok(IbcReceiveResponse::new(make_ack_fail(error.to_string()))
            .add_attribute("method", "ibc_packet_receive")
            .add_attribute("error", error.to_string())),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_ack(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    Ok(IbcBasicResponse::new())
}
pub fn do_ibc_packet_receive(
    mut deps: DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, ContractError> {
    let channel = msg.clone().packet.dest.channel_id;
    let chain = CHANNEL_TO_CHAIN
        .may_load(deps.storage, channel.as_str())?
        .ok_or(ContractError::Unauthorized {})?;
    let channel_info = CHAIN_TO_CHANNEL
        .may_load(deps.storage, chain.as_str())?
        .ok_or(ContractError::Unauthorized {})?;

    let packet_msg: IbcExecuteMsg = from_json(&msg.packet.data)?;
    let mut execute_env = ExecuteContext::new(
        deps.branch(),
        MessageInfo {
            funds: vec![],
            sender: Addr::unchecked(
                "cosmwasm122xa328nvn93rsemr980psc9m9qwh8xj8rdje4qtp68m5tyt7yusajjrpz",
            ),
        },
        env.clone(),
    );
    match packet_msg {
        IbcExecuteMsg::SendMessage { amp_packet } => {
            // Try to pull the username's address from the packet
            match amp_packet.ctx.get_origin_username() {
                Some(username) => {
                    // Check if username is registered
                    let vfs_address = KERNEL_ADDRESSES.load(execute_env.deps.storage, VFS_KEY)?;
                    let username_addr = AOSQuerier::get_address_from_username(
                        &execute_env.deps.querier,
                        &vfs_address,
                        username.as_str(),
                    )?;
                    if let Some(addr) = username_addr {
                        let new_amp_packet =
                            AMPPkt::new(addr, env.contract.address, amp_packet.messages.clone());
                        execute_env.amp_ctx = Some(new_amp_packet.clone());
                    }
                }
                None => {
                    execute_env.amp_ctx = None;
                }
            }

            let res = execute::send(execute_env, amp_packet.messages.first().unwrap().clone())?;

            Ok(IbcReceiveResponse::new(make_ack_success())
                .add_attributes(res.attributes)
                .add_submessages(res.messages)
                .add_events(res.events))
        }
        IbcExecuteMsg::SendMessageWithFunds {
            recipient,
            message,
            funds,
            original_sender,
            original_sender_username,
            previous_hops,
        } => {
            // Ensure the first message has funds
            ensure!(!funds.amount.is_zero(), ContractError::InvalidZeroAmount {});
            execute_env.info = MessageInfo {
                funds: vec![funds.clone()],
                sender: env.contract.address.clone(),
            };
            // Attempt to fetch the address for the original sender by their username
            let vfs_address = KERNEL_ADDRESSES.load(execute_env.deps.storage, VFS_KEY)?;
            let username_addr = if let Some(username) = original_sender_username.clone() {
                AOSQuerier::get_address_from_username(
                    &execute_env.deps.querier,
                    &vfs_address,
                    username.as_str(),
                )?
            } else {
                None
            };
            let msg = AMPMsg::new(recipient.clone(), message, Some(vec![funds.clone()]));
            match username_addr {
                Some(addr) => {
                    // Add potential username to the context
                    let mut ctx = AMPCtx::new(addr, env.contract.address, original_sender_username);
                    // Add previous hops to the context
                    for hop in previous_hops {
                        ctx.add_hop(hop);
                    }

                    let amp_packet = AMPPkt::new_with_ctx(ctx, vec![msg.clone()]);
                    execute_env.amp_ctx = Some(amp_packet.clone());
                }
                // If the original sender does not have a username registered on this chain we cannot use AMP
                None => {
                    execute_env.amp_ctx = None;
                }
            }

            let res = execute::send(execute_env, msg)?;

            // Refunds must be done via the ICS20 channel
            let ics20_channel_id = channel_info.ics20_channel_id.ok_or(ContractError::new(
                "Cannot refund, ICS20 Channel ID not set",
            ))?;
            // Save refund info
            REFUND_DATA.save(
                deps.storage,
                &RefundData {
                    original_sender,
                    funds,
                    channel: ics20_channel_id,
                },
            )?;
            Ok(IbcReceiveResponse::new(make_ack_success())
                .add_attribute("recipient", recipient.as_str())
                .add_attributes(res.attributes)
                .add_submessage(SubMsg::reply_always(
                    res.messages.first().unwrap().msg.clone(),
                    ReplyId::IBCTransferWithMsg.repr(),
                ))
                .add_events(res.events))
        }
        IbcExecuteMsg::CreateADO {
            instantiation_msg,
            owner,
            ado_type,
        } => ibc_create_ado(execute_env, owner, ado_type, instantiation_msg),
        IbcExecuteMsg::RegisterUsername { username, address } => {
            ibc_register_username(execute_env, username, address)
        }
    }
}

pub fn ibc_create_ado(
    execute_ctx: ExecuteContext,
    owner: AndrAddr,
    ado_type: String,
    msg: Binary,
) -> Result<IbcReceiveResponse, ContractError> {
    let res = execute::create(execute_ctx, ado_type, msg, Some(owner), None)?;
    Ok(IbcReceiveResponse::new(make_ack_success())
        .add_attributes(res.attributes)
        .add_events(res.events)
        .add_submessages(res.messages))
}

pub fn ibc_register_username(
    execute_ctx: ExecuteContext,
    username: String,
    addr: String,
) -> Result<IbcReceiveResponse, ContractError> {
    let vfs_address = KERNEL_ADDRESSES.load(execute_ctx.deps.storage, VFS_KEY)?;
    let msg = VFSExecuteMsg::RegisterUser {
        username,
        address: Some(execute_ctx.deps.api.addr_validate(&addr)?),
    };
    let sub_msg: SubMsg<Empty> = SubMsg::reply_on_error(
        WasmMsg::Execute {
            contract_addr: vfs_address.to_string(),
            msg: to_json_binary(&msg)?,
            funds: vec![],
        },
        ReplyId::RegisterUsername.repr(),
    );
    Ok(IbcReceiveResponse::new(make_ack_success()).add_submessage(sub_msg))
}

pub fn validate_order_and_version(
    channel: &IbcChannel,
    counterparty_version: Option<&str>,
) -> Result<Option<Ibc3ChannelOpenResponse>, ContractError> {
    // We expect an unordered channel here. Ordered channels have the
    // property that if a message is lost the entire channel will stop
    // working until you start it again.
    ensure!(
        channel.order == IbcOrder::Unordered,
        ContractError::OrderedChannel {}
    );
    ensure!(
        channel.version == IBC_VERSION || channel.version == "ics20-1",
        ContractError::InvalidVersion {
            actual: channel.version.to_string(),
            expected: IBC_VERSION.to_string(),
        }
    );

    // Make sure that we're talking with a counterparty who speaks the
    // same "protocol" as us.
    //
    // For a connection between chain A and chain B being established
    // by chain A, chain B knows counterparty information during
    // `OpenTry` and chain A knows counterparty information during
    // `OpenAck`. We verify it when we have it but when we don't it's
    // alright.
    if let Some(counterparty_version) = counterparty_version {
        ensure!(
            counterparty_version == IBC_VERSION || channel.version == "ics20-1",
            ContractError::InvalidVersion {
                actual: counterparty_version.to_string(),
                expected: IBC_VERSION.to_string(),
            }
        );
    }

    Ok(Some(Ibc3ChannelOpenResponse {
        version: channel.version.to_string(),
    }))
}

pub fn get_counterparty_denom(
    deps: &Deps,
    denom: &str,
    src_channel: &str,
) -> Result<(String, DenomInfo), ContractError> {
    // if denom is ibc denom, get denom trace
    let denom_trace = if denom.starts_with("ibc/") {
        let ibc_registry_addr = KERNEL_ADDRESSES.load(deps.storage, IBC_REGISTRY_KEY)?;
        AOSQuerier::denom_trace_getter(&deps.querier, &ibc_registry_addr, denom)?
    } else {
        // if not ibc denom, use base denom
        DenomInfo::new(denom.to_string(), "".to_string())
    };

    let (counterparty_denom, counterparty_denom_trace) =
        AOSQuerier::get_counterparty_denom(&deps.querier, &denom_trace, src_channel)?;
    Ok((counterparty_denom, counterparty_denom_trace))
}
