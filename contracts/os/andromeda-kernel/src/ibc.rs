use crate::ack::{make_ack_fail, make_ack_success};
use crate::execute;
use crate::proto::{DenomTrace, MsgTransfer, QueryDenomTraceRequest};
use crate::state::{CHANNEL_TO_CHAIN, KERNEL_ADDRESSES, REFUND_DATA};
use andromeda_std::amp::VFS_KEY;
use andromeda_std::common::context::ExecuteContext;
use andromeda_std::common::reply::ReplyId;
use andromeda_std::error::{ContractError, Never};
use andromeda_std::os::kernel::RefundData;
use andromeda_std::{
    amp::{messages::AMPMsg, AndrAddr},
    os::{kernel::IbcExecuteMsg, vfs::ExecuteMsg as VFSExecuteMsg},
};
use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, from_json, to_json_binary, Addr, Binary, Coin, Deps, DepsMut, Empty, Env,
    Ibc3ChannelOpenResponse, IbcBasicResponse, IbcChannel, IbcChannelCloseMsg,
    IbcChannelConnectMsg, IbcChannelOpenMsg, IbcOrder, IbcPacketAckMsg, IbcPacketReceiveMsg,
    IbcPacketTimeoutMsg, IbcReceiveResponse, MessageInfo, SubMsg, Timestamp, WasmMsg,
};
use itertools::Itertools;
use sha256::digest;

pub const IBC_VERSION: &str = "andr-kernel-1";
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
        Err(error) => Ok(IbcReceiveResponse::new()
            .add_attribute("method", "ibc_packet_receive")
            .add_attribute("error", error.to_string())
            .set_ack(make_ack_fail(error.to_string()))),
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
    ensure!(
        CHANNEL_TO_CHAIN.has(deps.storage, channel.as_str()),
        ContractError::Unauthorized {}
    );
    let packet_msg: IbcExecuteMsg = from_json(&msg.packet.data)?;
    let mut execute_env = ExecuteContext {
        env: env.clone(),
        deps: deps.branch(),
        info: MessageInfo {
            funds: vec![],
            sender: Addr::unchecked("foreign_kernel"),
        },
        amp_ctx: None,
    };
    match packet_msg {
        IbcExecuteMsg::SendMessage { amp_packet } => {
            execute_env.amp_ctx = Some(amp_packet.clone());

            let res = execute::send(execute_env, amp_packet.messages.first().unwrap().clone())?;

            Ok(IbcReceiveResponse::new()
                .set_ack(make_ack_success())
                .add_attributes(res.attributes)
                .add_submessages(res.messages)
                .add_events(res.events))
        }
        IbcExecuteMsg::SendMessageWithFunds {
            recipient,
            message,
            funds,
            original_sender,
        } => {
            let amp_msg = AMPMsg::new(recipient, message.clone(), Some(vec![funds.clone()]));

            execute_env.info = MessageInfo {
                funds: vec![funds.clone()],
                sender: Addr::unchecked("foreign_kernel"),
            };
            let res = execute::send(execute_env, amp_msg)?;

            // Save refund info
            REFUND_DATA.save(
                deps.storage,
                &RefundData {
                    original_sender,
                    funds,
                    channel,
                },
            )?;
            Ok(IbcReceiveResponse::new()
                .set_ack(make_ack_success())
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
    _execute_ctx: ExecuteContext,
    _owner: AndrAddr,
    _ado_type: String,
    _msg: Binary,
) -> Result<IbcReceiveResponse, ContractError> {
    Err(ContractError::CrossChainComponentsCurrentlyDisabled {})
    // let res = execute::create(execute_env, ado_type, msg, Some(owner), None)?;
    // Ok(IbcReceiveResponse::new()
    //     .add_attributes(res.attributes)
    //     .add_events(res.events)
    //     .add_submessages(res.messages)
    //     .set_ack(make_ack_create_ado_success()))
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
    Ok(IbcReceiveResponse::new()
        .add_submessage(sub_msg)
        .set_ack(make_ack_success()))
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

// IBC transfer port
const TRANSFER_PORT: &str = "transfer";

fn generate_ibc_denom(channel: String, denom: String) -> String {
    let path = format!("{TRANSFER_PORT}/{channel}/{denom}");
    format!("ibc/{}", digest(path).to_uppercase())
}

#[allow(clippy::too_many_arguments)]
pub fn generate_transfer_message(
    deps: &Deps,
    recipient: AndrAddr,
    message: Binary,
    funds: Coin,
    channel: String,
    from_addr: String,
    to_addr: String,
    time: Timestamp,
) -> Result<MsgTransfer, ContractError> {
    // Convert funds denom
    let new_denom = if funds.denom.starts_with("ibc/") {
        let hops = unwrap_denom_path(deps, &funds.denom)?;
        /*
        Hops are ordered from most recent hop to the first hop, we check if we're unwrapping by checking the channel of the most recent hop.
        If the channels match we're unwrapping and the receiving denom is the local denom of the previous hop (hop[1]).
        Otherwise we're wrapping and we proceed as expected.
        */
        if !hops[0].on.eq(&Some(channel.clone())) {
            generate_ibc_denom(channel.clone(), hops[0].local_denom.clone())
        } else {
            hops[1].local_denom.clone()
        }
    } else {
        generate_ibc_denom(channel.clone(), funds.clone().denom)
    };
    let new_coin = Coin::new(funds.amount.u128(), new_denom);
    let msg = AMPMsg::new(recipient.get_raw_path(), message, Some(vec![new_coin]));
    let serialized = msg.to_ibc_hooks_memo(to_addr.clone(), from_addr.clone());

    let ts = time.plus_seconds(PACKET_LIFETIME);

    Ok(MsgTransfer {
        source_port: TRANSFER_PORT.into(),
        source_channel: channel,
        token: Some(funds.into()),
        sender: from_addr,
        receiver: to_addr,
        timeout_height: None,
        timeout_timestamp: Some(ts.nanos()),
        memo: serialized,
    })
}

// Methods adapted from Osmosis Registry contract found here:
// https://github.com/osmosis-labs/osmosis/blob/main/cosmwasm/packages/registry/src/registry.rs#L14
#[cw_serde]
pub struct MultiHopDenom {
    pub local_denom: String,
    pub on: Option<String>,
}

pub fn hash_denom_trace(unwrapped: &str) -> String {
    format!("ibc/{}", sha256::digest(unwrapped))
}

pub fn unwrap_denom_path(deps: &Deps, denom: &str) -> Result<Vec<MultiHopDenom>, ContractError> {
    // Check that the denom is an IBC denom
    if !denom.starts_with("ibc/") {
        return Ok(vec![MultiHopDenom {
            local_denom: denom.to_string(),
            on: None,
        }]);
    }

    // Get the denom trace
    let res = QueryDenomTraceRequest {
        hash: denom.to_string(),
    }
    .query(&deps.querier)
    .map_err(|_| ContractError::InvalidDenomTrace {
        denom: denom.to_string(),
    })?;

    let DenomTrace { path, base_denom } = match res.denom_trace {
        Some(denom_trace) => Ok(denom_trace),
        None => Err(ContractError::InvalidDenomTrace {
            denom: denom.into(),
        }),
    }?;

    deps.api.debug(&format!("procesing denom trace {path}"));
    // Let's iterate over the parts of the denom trace and extract the
    // chain/channels into a more useful structure: MultiHopDenom
    let mut hops: Vec<MultiHopDenom> = vec![];
    let mut rest: &str = &path;
    let parts = path.split('/');

    for chunk in &parts.chunks(2) {
        let Some((port, channel)) = chunk.take(2).collect_tuple() else {
            return Err(ContractError::InvalidDenomTracePath {
                path: path.clone(),
                denom: denom.into(),
            });
        };

        // Check that the port is "transfer"
        if port != TRANSFER_PORT {
            return Err(ContractError::InvalidTransferPort { port: port.into() });
        }

        // Check that the channel is valid
        let full_trace = rest.to_owned() + "/" + &base_denom;
        hops.push(MultiHopDenom {
            local_denom: hash_denom_trace(&full_trace),
            on: Some(channel.to_string()),
        });

        rest = rest
            .trim_start_matches(&format!("{port}/{channel}"))
            .trim_start_matches('/'); // hops other than first and last will have this slash
    }

    hops.push(MultiHopDenom {
        local_denom: base_denom,
        on: None,
    });

    Ok(hops)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ibc_denom() {
        let channel = "channel-141";
        let denom = "uosmo";

        let expected = "ibc/14F9BC3E44B8A9C1BE1FB08980FAB87034C9905EF17CF2F5008FC085218811CC";
        let res = generate_ibc_denom(channel.to_string(), denom.to_string());

        assert_eq!(expected, res)
    }
}
