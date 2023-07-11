use andromeda_std::amp::addresses::AndrAddr;
use andromeda_std::amp::messages::AMPMsgConfig;
// use andromeda_std::os::messages::extract_chain;
use andromeda_std::amp::messages::AMPMsg;
use andromeda_std::error::ContractError;
use andromeda_std::ibc::message_bridge::ExecuteMsg as BridgeExecuteMsg;
use cosmwasm_std::{to_binary, Addr, Binary, Coin, CosmosMsg, ReplyOn, Storage, SubMsg, WasmMsg};
use cw_storage_plus::Map;

pub const IBC_BRIDGE: &str = "ibc-bridge";
pub const WORMHOLE_BRIDGE: &str = "wormhole-bridge";

pub const KERNEL_ADDRESSES: Map<&str, Addr> = Map::new("kernel_addresses");

#[allow(clippy::too_many_arguments)]
pub fn parse_path_direct(
    recipient: AndrAddr,
    message: Binary,
    funds: Vec<Coin>,
    storage: &dyn Storage,
    reply_on: Option<ReplyOn>,
    exit_at_error: Option<bool>,
    gas_limit: Option<u64>,
) -> Result<Option<SubMsg>, ContractError> {
    if recipient.is_vfs_path() {
        let protocol: Option<&str> = recipient.get_protocol();
        if protocol.is_some() {
            match protocol {
                // load vector of supported chains
                // load bridge contract address
                // extract message from path

                // Will import the bridge's execute msg once merged
                Some("ibc") => {
                    let raw_path = recipient.get_raw_path();
                    Ok(Some(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: KERNEL_ADDRESSES.load(storage, IBC_BRIDGE)?.to_string(),
                        msg: to_binary(&BridgeExecuteMsg::SendAmpPacket {
                            chain: recipient.get_chain().unwrap_or_default().to_owned(),
                            message: vec![AMPMsg::new(raw_path, message, Some(funds.clone()))
                                .with_config(AMPMsgConfig::new(
                                    reply_on,
                                    exit_at_error,
                                    gas_limit,
                                ))],
                        })?,
                        funds,
                    }))))
                }
                Some("wormhole") => {
                    let raw_path = recipient.get_raw_path();
                    Ok(Some(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: KERNEL_ADDRESSES.load(storage, WORMHOLE_BRIDGE)?.to_string(),
                        msg: to_binary(&BridgeExecuteMsg::SendAmpPacket {
                            chain: recipient.get_chain().unwrap_or_default().to_owned(),
                            message: vec![AMPMsg::new(raw_path, message, Some(funds.clone()))
                                .with_config(AMPMsgConfig::new(
                                    reply_on,
                                    exit_at_error,
                                    gas_limit,
                                ))],
                        })?,
                        funds,
                    }))))
                }
                _ => Err(ContractError::UnsupportedProtocol {}),
            }
        } else {
            // In case there's no protocol, the pathname should look like this : chain/path or just /path
            let chain = recipient.get_chain();
            match chain {
                // In case of andromeda we proceed as usual
                // This approach assumes that andromeda's always the native chain
                Some("andr") => Ok(None),
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
                                    chain: chain.to_owned(),
                                    message: vec![AMPMsg::new(
                                        recipient,
                                        message,
                                        Some(funds.clone()),
                                    )
                                    .with_config(AMPMsgConfig::new(
                                        reply_on,
                                        exit_at_error,
                                        gas_limit,
                                    ))],
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

#[allow(clippy::too_many_arguments)]
pub fn parse_path_direct_no_ctx(
    recipient: AndrAddr,
    message: Binary,
    funds: Vec<Coin>,
    storage: &dyn Storage,
) -> Result<Option<SubMsg>, ContractError> {
    if recipient.is_vfs_path() {
        let protocol: Option<&str> = recipient.get_protocol();
        if protocol.is_some() {
            match protocol {
                // load vector of supported chains
                // load bridge contract address
                // extract message from path

                // Will import the bridge's execute msg once merged
                Some("ibc") => {
                    let raw_path = recipient.get_raw_path();
                    Ok(Some(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: KERNEL_ADDRESSES.load(storage, IBC_BRIDGE)?.to_string(),
                        msg: to_binary(&BridgeExecuteMsg::SendAmpPacket {
                            chain: recipient.get_chain().unwrap_or_default().to_owned(),
                            message: vec![AMPMsg::new(raw_path, message, Some(funds.clone()))],
                        })?,
                        funds,
                    }))))
                }
                Some("wormhole") => {
                    let raw_path = recipient.get_raw_path();
                    Ok(Some(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: KERNEL_ADDRESSES.load(storage, WORMHOLE_BRIDGE)?.to_string(),
                        msg: to_binary(&BridgeExecuteMsg::SendAmpPacket {
                            chain: recipient.get_chain().unwrap_or_default().to_owned(),
                            message: vec![AMPMsg::new(raw_path, message, Some(funds.clone()))],
                        })?,
                        funds,
                    }))))
                }
                _ => Err(ContractError::UnsupportedProtocol {}),
            }
        } else {
            // In case there's no protocol, the pathname should look like this : chain/path or just /path
            let chain = recipient.get_chain();
            match chain {
                // In case of andromeda we proceed as usual
                // This approach assumes that andromeda's always the native chain
                Some("andr") => Ok(None),
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
                                    chain: chain.to_owned(),
                                    message: vec![AMPMsg::new(
                                        recipient,
                                        message,
                                        Some(funds.clone()),
                                    )],
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
