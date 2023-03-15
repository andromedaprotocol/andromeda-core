use andromeda_os::messages::{extract_chain, AMPMsg};
use common::error::ContractError;
use cosmwasm_std::{to_binary, Addr, Api, Binary, Coin, CosmosMsg, Storage, SubMsg, WasmMsg};
use cw_storage_plus::{Item, Map};

pub const ADO_DB_KEY: &str = "adodb";
pub const VFS_KEY: &str = "vfs";
pub const IBC_BRIDGE: &str = "ibc-bridge";
pub const WORMHOLE_BRIDGE: &str = "wormhole-bridge";

pub const KERNEL_ADDRESSES: Map<&str, Addr> = Map::new("kernel_addresses");

pub fn parse_path(
    recipient: String,
    message: Binary,
    funds: Vec<Coin>,
    storage: &dyn Storage,
    api: &dyn Api,
) -> Result<Option<SubMsg>, ContractError> {
    if recipient.contains('/') {
        let pathname = &recipient;
        let protocol: Option<&str> = if let Some(idx) = pathname.find(":") {
            let protocol = &pathname[..idx];
            Some(protocol)
        } else {
            None
        };
        let binary_message = message.clone();
        let funds = funds.clone();
        if protocol.is_some() {
            match protocol {
                // load vector of supported chains
                // load bridge contract address
                // extract message from path
                Some("ibc") => Ok(Some(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: KERNEL_ADDRESSES.load(storage, IBC_BRIDGE)?.to_string(),
                    msg: to_binary(&BridgeExecuteMsg::SendMessage {
                        chain: extract_chain(&pathname),
                        recipient,
                        message: binary_message,
                    })?,
                    funds,
                })))),
                Some("wormhole") => Ok(Some(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: KERNEL_ADDRESSES.load(storage, WORMHOLE_BRIDGE)?.to_string(),
                    msg: binary_message,
                    funds,
                })))),
                _ => Err(ContractError::UnsupportedOperation {}),
            }
        } else {
            let chain = pathname.splitn(2, '/').next();
            match chain {
                // In case of andromeda we proceed as usual
                Some("andromeda") => Ok(None),
                // In case of other chain, we forward to bridge contract
                Some(chain) => Ok(Some(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: KERNEL_ADDRESSES.load(storage, IBC_BRIDGE)?.to_string(),
                    msg: binary_message,
                    funds,
                })))),
                // If no identifiable chain, assume it's the implicit path, so proceed to querying vfs to resolve path
                None => Ok(None),
            }
        }
    } else {
        Ok(None)
    }
}
