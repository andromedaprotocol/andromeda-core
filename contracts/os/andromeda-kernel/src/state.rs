// use andromeda_os::messages::extract_chain;
use andromeda_ibc::message_bridge::ExecuteMsg as BridgeExecuteMsg;
use andromeda_os::messages::{extract_chain, AMPMsg, AMPPkt};
use common::error::ContractError;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, CosmosMsg, Env, MessageInfo, ReplyOn, Storage, SubMsg, WasmMsg,
};
use cw_storage_plus::Map;

pub const ADO_DB_KEY: &str = "adodb";
pub const VFS_KEY: &str = "vfs";
pub const IBC_BRIDGE: &str = "ibc-bridge";
pub const WORMHOLE_BRIDGE: &str = "wormhole-bridge";

pub const KERNEL_ADDRESSES: Map<&str, Addr> = Map::new("kernel_addresses");

// turns ibc://juno/path into /path
fn adjust_recipient_with_protocol(recipient: &str) -> String {
    let mut count_slashes = 0;
    let mut last_slash_index = 0;

    // Iterate through each character in the input string
    for (i, c) in recipient.chars().enumerate() {
        // If the current character is a slash
        if c == '/' {
            count_slashes += 1;
            last_slash_index = i;

            // If we've found the third slash, exit the loop
            if count_slashes == 3 {
                break;
            }
        }
    }

    // Return the substring starting from the last slash index
    recipient[last_slash_index..].to_owned()
}

pub fn parse_path(
    recipient: String,
    packet: AMPPkt,
    amp_message: AMPMsg,
    storage: &dyn Storage,
) -> Result<Option<SubMsg>, ContractError> {
    if recipient.contains('/') {
        let pathname = &recipient;
        let protocol: Option<&str> = if let Some(idx) = pathname.find(':') {
            let protocol = &pathname[..idx];
            Some(protocol)
        } else {
            None
        };
        let binary_message = amp_message.message;
        let funds = amp_message.funds;

        if protocol.is_some() {
            match protocol {
                // load vector of supported chains
                // load bridge contract address
                // extract message from path

                // Will import the bridge's execute msg once merged
                Some("ibc") => {
                    let recipient = adjust_recipient_with_protocol(&recipient);
                    Ok(Some(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: KERNEL_ADDRESSES.load(storage, IBC_BRIDGE)?.to_string(),
                        msg: to_binary(&BridgeExecuteMsg::SendAmpPacket {
                            chain: extract_chain(pathname).unwrap_or_default().to_owned(),
                            message: to_binary(&AMPPkt::new(
                                packet.get_origin(),
                                packet.get_previous_sender(),
                                vec![AMPMsg::new(
                                    recipient,
                                    binary_message,
                                    Some(funds.clone()),
                                    Some(amp_message.reply_on),
                                    Some(amp_message.exit_at_error),
                                    amp_message.gas_limit,
                                )],
                            ))?,
                        })?,
                        funds,
                    }))))
                }
                Some("wormhole") => {
                    let recipient = adjust_recipient_with_protocol(&recipient);
                    Ok(Some(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: KERNEL_ADDRESSES.load(storage, WORMHOLE_BRIDGE)?.to_string(),
                        msg: to_binary(&BridgeExecuteMsg::SendAmpPacket {
                            chain: extract_chain(pathname).unwrap_or_default().to_owned(),
                            message: to_binary(&AMPPkt::new(
                                packet.get_origin(),
                                packet.get_previous_sender(),
                                vec![AMPMsg::new(
                                    recipient,
                                    binary_message,
                                    Some(funds.clone()),
                                    Some(amp_message.reply_on),
                                    Some(amp_message.exit_at_error),
                                    amp_message.gas_limit,
                                )],
                            ))?,
                        })?,
                        funds,
                    }))))
                }
                _ => Err(ContractError::UnsupportedProtocol {}),
            }
        } else {
            // In case there's no protocol, the pathname should look like this : chain/path or just /path
            let chain = pathname.split('/').next();
            match chain {
                // In case of andromeda we proceed as usual
                // This approach assumes that andromeda's always the native chain
                Some("andromeda") => Ok(None),
                // In case of other chain, we forward to bridge contract
                Some(chain) => {
                    if chain.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: KERNEL_ADDRESSES.load(storage, IBC_BRIDGE)?.to_string(),
                            msg: to_binary(&BridgeExecuteMsg::SendAmpPacket {
                                chain: extract_chain(pathname).unwrap_or_default().to_owned(),
                                message: to_binary(&AMPPkt::new(
                                    packet.get_origin(),
                                    packet.get_previous_sender(),
                                    vec![AMPMsg::new(
                                        recipient,
                                        binary_message,
                                        Some(funds.clone()),
                                        Some(amp_message.reply_on),
                                        Some(amp_message.exit_at_error),
                                        amp_message.gas_limit,
                                    )],
                                ))?,
                            })?,
                            funds,
                        }))))
                    }
                }
                None => Err(ContractError::InvalidPathname { error: None }),
            }
        }
    } else {
        Ok(None)
    }
}
#[allow(clippy::too_many_arguments)]
pub fn parse_path_direct(
    env: Env,
    info: MessageInfo,
    recipient: String,
    message: Binary,
    funds: Vec<Coin>,
    storage: &dyn Storage,
    reply_on: Option<ReplyOn>,
    exit_at_error: Option<bool>,
    gas_limit: Option<u64>,
) -> Result<Option<SubMsg>, ContractError> {
    if recipient.contains('/') {
        let pathname = &recipient;
        let protocol: Option<&str> = if let Some(idx) = pathname.find(':') {
            let protocol = &pathname[..idx];
            Some(protocol)
        } else {
            None
        };
        let funds = funds;
        if protocol.is_some() {
            match protocol {
                // load vector of supported chains
                // load bridge contract address
                // extract message from path

                // Will import the bridge's execute msg once merged
                Some("ibc") => {
                    let recipient = adjust_recipient_with_protocol(&recipient);
                    Ok(Some(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: KERNEL_ADDRESSES.load(storage, IBC_BRIDGE)?.to_string(),
                        msg: to_binary(&BridgeExecuteMsg::SendAmpPacket {
                            chain: extract_chain(pathname).unwrap_or_default().to_owned(),
                            message: to_binary(&AMPPkt::new(
                                info.sender,
                                env.contract.address,
                                vec![AMPMsg::new(
                                    recipient,
                                    message,
                                    Some(funds.clone()),
                                    reply_on,
                                    exit_at_error,
                                    gas_limit,
                                )],
                            ))?,
                        })?,
                        funds,
                    }))))
                }
                Some("wormhole") => {
                    let recipient = adjust_recipient_with_protocol(&recipient);
                    Ok(Some(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: KERNEL_ADDRESSES.load(storage, WORMHOLE_BRIDGE)?.to_string(),
                        msg: to_binary(&BridgeExecuteMsg::SendAmpPacket {
                            chain: extract_chain(pathname).unwrap_or_default().to_owned(),
                            message: to_binary(&AMPPkt::new(
                                info.sender,
                                env.contract.address,
                                vec![AMPMsg::new(
                                    recipient,
                                    message,
                                    Some(funds.clone()),
                                    reply_on,
                                    exit_at_error,
                                    gas_limit,
                                )],
                            ))?,
                        })?,
                        funds,
                    }))))
                }
                _ => Err(ContractError::UnsupportedProtocol {}),
            }
        } else {
            // In case there's no protocol, the pathname should look like this : chain/path or just /path
            let chain = pathname.split('/').next();
            match chain {
                // In case of andromeda we proceed as usual
                // This approach assumes that andromeda's always the native chain
                Some("andromeda") => Ok(None),
                // In case of other chain, we forward to bridge contract
                Some(chain) => {
                    if chain.is_empty() {
                        Ok(None)
                    } else {
                        {
                            Ok(Some(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                                contract_addr: KERNEL_ADDRESSES
                                    .load(storage, IBC_BRIDGE)?
                                    .to_string(),
                                msg: to_binary(&BridgeExecuteMsg::SendAmpPacket {
                                    chain: extract_chain(pathname).unwrap_or_default().to_owned(),
                                    message: to_binary(&AMPPkt::new(
                                        info.sender,
                                        env.contract.address,
                                        vec![AMPMsg::new(
                                            recipient,
                                            message,
                                            Some(funds.clone()),
                                            reply_on,
                                            exit_at_error,
                                            gas_limit,
                                        )],
                                    ))?,
                                })?,
                                funds,
                            }))))
                        }
                    }
                }
                None => Err(ContractError::InvalidPathname { error: None }),
            }
        }
    } else {
        Ok(None)
    }
}
