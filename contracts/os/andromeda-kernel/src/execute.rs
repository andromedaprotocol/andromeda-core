use andromeda_std::ado_contract::ADOContract;
use andromeda_std::amp::addresses::AndrAddr;
use andromeda_std::amp::messages::{AMPMsg, AMPPkt};
use andromeda_std::amp::{ADO_DB_KEY, VFS_KEY};

use andromeda_std::common::context::ExecuteContext;
use andromeda_std::error::ContractError;
use andromeda_std::ibc::message_bridge::IbcExecuteMsg;
use andromeda_std::os::aos_querier::AOSQuerier;

use cosmwasm_std::{
    attr, ensure, to_binary, BankMsg, Binary, CosmosMsg, IbcMsg, Response, SubMsg, WasmMsg,
};

use crate::ibc::{generate_transfer_message, PACKET_LIFETIME};
use crate::state::{ChannelInfo, ADO_OWNER, CHANNELS, KERNEL_ADDRESSES};
use crate::{query, reply::ReplyId};

pub fn send(execute_env: ExecuteContext, message: AMPMsg) -> Result<Response, ContractError> {
    let res = MsgHandler::new(message).handle(&execute_env, 0)?;

    Ok(res)
}

pub fn amp_receive(execute_env: ExecuteContext, packet: AMPPkt) -> Result<Response, ContractError> {
    ensure!(
        query::verify_address(
            execute_env.deps.as_ref(),
            execute_env.info.sender.to_string(),
        )? || packet.ctx.get_origin() == execute_env.info.sender,
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
        let handler = MsgHandler::new(message.clone());
        res = handler.handle(&execute_env, idx as u64)?;
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
) -> Result<Response, ContractError> {
    let vfs_addr = KERNEL_ADDRESSES.load(execute_env.deps.storage, VFS_KEY)?;
    let adodb_addr = KERNEL_ADDRESSES.load(execute_env.deps.storage, ADO_DB_KEY)?;

    let ado_owner = owner.unwrap_or(AndrAddr::from_string(execute_env.info.sender.to_string()));
    let owner_addr = ado_owner.get_raw_address_from_vfs(&execute_env.deps.as_ref(), vfs_addr)?;
    let code_id = AOSQuerier::code_id_getter(&execute_env.deps.querier, &adodb_addr, &ado_type)?;
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

    let channel_info = ChannelInfo {
        ics20_channel_id,
        direct_channel_id,
        kernel_address,
        supported_modules: vec![],
    };
    CHANNELS.save(execute_env.deps.storage, &chain, &channel_info)?;

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

/// Handles a given AMP message and returns a response
///
/// Separated due to common functionality across multiple messages
struct MsgHandler(AMPMsg);

impl MsgHandler {
    pub fn new(msg: AMPMsg) -> Self {
        Self(msg)
    }

    fn message(&self) -> &AMPMsg {
        &self.0
    }

    #[inline]
    pub fn handle(
        &self,
        execute_env: &ExecuteContext,
        sequence: u64,
    ) -> Result<Response, ContractError> {
        let protocol = self.message().recipient.get_protocol();

        match protocol {
            Some("ibc") => self.handle_ibc(execute_env, sequence),
            _ => self.handle_local(execute_env, sequence),
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
        execute_env: &ExecuteContext,
        sequence: u64,
    ) -> Result<Response, ContractError> {
        let mut res = Response::default();
        let AMPMsg {
            message,
            recipient,
            funds,
            ..
        } = self.message();
        let recipient_addr = recipient.get_raw_address(&execute_env.deps.as_ref())?;

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

            res = res
                .add_submessage(SubMsg::reply_on_error(CosmosMsg::Bank(sub_msg), 1))
                .add_attributes(vec![
                    attr(format!("recipient:{sequence}"), recipient_addr),
                    attr(format!("bank_send_amount:{sequence}"), funds[0].to_string()),
                ]);
        } else {
            let origin = if let Some(amp_ctx) = execute_env.amp_ctx.clone() {
                amp_ctx.ctx.get_origin()
            } else {
                execute_env.info.sender.to_string()
            };
            let previous_sender = execute_env.info.sender.to_string();

            let amp_msg = AMPMsg::new(
                recipient_addr.clone(),
                message.clone(),
                Some(vec![funds[0].clone()]),
            );

            let new_packet = AMPPkt::new(origin, previous_sender, vec![amp_msg]);

            let sub_msg = new_packet.to_sub_msg(
                recipient_addr.clone(),
                Some(vec![funds[0].clone()]),
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
        execute_env: &ExecuteContext,
        sequence: u64,
    ) -> Result<Response, ContractError> {
        if let Some(chain) = self.message().recipient.get_chain() {
            let channel_info =
                if let Some(channel_info) = CHANNELS.may_load(execute_env.deps.storage, chain)? {
                    Ok::<ChannelInfo, ContractError>(channel_info)
                } else {
                    return Err(ContractError::InvalidPacket {
                        error: Some(format!("Channel not found for chain {chain}")),
                    });
                }?;
            if !self.message().funds.is_empty() {
                self.handle_ibc_hooks(execute_env, sequence, channel_info)
            } else {
                self.handle_ibc_direct(execute_env, sequence, channel_info)
            }
        } else {
            Err(ContractError::InvalidPacket {
                error: Some("Chain not provided".to_string()),
            })
        }
    }

    fn handle_ibc_direct(
        &self,
        execute_env: &ExecuteContext,
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
            timeout: execute_env
                .env
                .block
                .time
                .plus_seconds(PACKET_LIFETIME)
                .into(),
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
        execute_env: &ExecuteContext,
        sequence: u64,
        channel_info: ChannelInfo,
    ) -> Result<Response, ContractError> {
        let AMPMsg {
            recipient,
            message,
            funds,
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
        let msg = generate_transfer_message(
            &execute_env.deps.as_ref(),
            recipient.clone(),
            message.clone(),
            msg_funds.clone(),
            channel.clone(),
            execute_env.env.contract.address.to_string(),
            channel_info.kernel_address.clone(),
            execute_env.env.block.time,
        )?;
        Ok(Response::default()
            .add_message(msg)
            .add_attribute(format!("method:{sequence}"), "execute_send_message")
            .add_attribute(format!("channel:{sequence}"), channel)
            .add_attribute(
                format!("receiving_kernel_address:{sequence}"),
                channel_info.kernel_address,
            )
            .add_attribute(format!("chain:{sequence}"), chain))
    }
}
