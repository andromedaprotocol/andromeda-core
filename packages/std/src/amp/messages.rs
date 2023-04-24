use crate::error::ContractError;
use crate::os::{adodb::QueryMsg as ADODBQueryMsg, kernel::QueryMsg as KernelQueryMsg};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, ContractInfoResponse, CosmosMsg, Deps, QueryRequest, ReplyOn,
    SubMsg, WasmMsg, WasmQuery,
};

use super::addresses::AndrAddr;

#[cw_serde]
pub enum ExecuteMsg {
    AMPReceive(AMPPkt),
}

#[cw_serde]
pub enum VFSQueryMsg {
    ResolvePath { path: String },
}
pub const ADO_DB_KEY: &str = "adodb";

#[cw_serde]
/// This struct defines how the kernel parses and relays messages between ADOs
/// It contains a simple recipient string which may use our namespacing implementation or a simple contract address
/// If the desired recipient is via IBC then namespacing must be employed
/// The attached message must be a binary encoded execute message for the receiving ADO
/// Funds can be attached for an individual message and will be attached accordingly
pub struct AMPMsg {
    /// The message recipient, can be a contract/wallet address or a namespaced URI
    pub recipient: AndrAddr,
    /// The message to be sent to the recipient
    pub message: Binary,
    /// Any funds to be attached to the message, defaults to an empty vector
    pub funds: Vec<Coin>,
    /// When the message should reply, defaults to Always
    pub reply_on: ReplyOn,
    /// Determines whether the operation should terminate or proceed upon a failed message
    pub exit_at_error: bool,
    /// An optional imposed gas limit for the message
    pub gas_limit: Option<u64>,
}

pub fn extract_chain(pathname: &str) -> Option<&str> {
    let start = pathname.find('/')? + 2;
    let end = pathname[start..]
        .find('/')
        .unwrap_or(pathname[start..].len());
    Some(&pathname[start..start + end])
}

impl AMPMsg {
    /// Creates a new AMPMsg
    pub fn new(
        recipient: impl Into<String>,
        message: Binary,
        funds: Option<Vec<Coin>>,
        reply_on: Option<ReplyOn>,
        exit_at_error: Option<bool>,
        gas_limit: Option<u64>,
    ) -> AMPMsg {
        AMPMsg {
            recipient: AndrAddr::from_string(recipient),
            message,
            funds: funds.unwrap_or_default(),
            reply_on: reply_on.unwrap_or(ReplyOn::Always),
            exit_at_error: exit_at_error.unwrap_or(true),
            gas_limit,
        }
    }

    /// Generates an AMPPkt containing the given AMPMsg
    pub fn generate_sub_message(
        &self,
        deps: &Deps,
        origin: String,
        previous_sender: String,
        id: u64,
    ) -> Result<SubMsg, ContractError> {
        let contract_addr = self.recipient.get_raw_address(deps)?;
        let pkt = AMPPkt::new(origin, previous_sender, vec![self.clone()]);
        let msg = to_binary(&ExecuteMsg::AMPReceive(pkt))?;
        Ok(SubMsg {
            id,
            reply_on: self.reply_on.clone(),
            gas_limit: self.gas_limit,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.into(),
                msg,
                funds: self.funds.to_vec(),
            }),
        })
    }
}

#[cw_serde]
/// Allows the user to choose between bypassing or using the kernel
pub enum MessagePath {
    Direct(),
    Kernel(ReplyGasExit),
}

#[cw_serde]
pub struct ReplyGasExit {
    pub reply_on: Option<ReplyOn>,
    pub gas_limit: Option<u64>,
    pub exit_at_error: Option<bool>,
}

#[cw_serde]
/// An Andromeda packet contains all message protocol related data, this is what is sent between ADOs when communicating
/// It contains an original sender, if used for authorisation the sender must be authorised
/// The previous sender is the one who sent the message
/// A packet may contain several messages which allows for message batching
pub struct AMPPkt {
    /// The original sender of the packet, immutable, can be retrieved with `AMPPkt.get_origin`
    origin: String,
    /// The previous sender of the packet, immutable, can be retrieved with `AMPPkt.get_previous_sender`
    previous_sender: String,
    /// Any messages associated with the packet
    pub messages: Vec<AMPMsg>,
}

impl AMPPkt {
    /// Creates a new AMP Packet
    pub fn new(
        origin: impl Into<String>,
        previous_sender: impl Into<String>,
        messages: Vec<AMPMsg>,
    ) -> AMPPkt {
        AMPPkt {
            origin: origin.into(),
            previous_sender: previous_sender.into(),
            messages,
        }
    }

    /// Adds a message to the current AMP Packet
    pub fn add_message(&mut self, message: AMPMsg) {
        self.messages.push(message)
    }

    /// Gets the original sender of a message
    pub fn get_origin(&self) -> String {
        self.origin.clone()
    }

    /// Gets the previous sender of a message
    pub fn get_previous_sender(&self) -> String {
        self.previous_sender.clone()
    }

    /// Gets all unique recipients for messages
    pub fn get_unique_recipients(&self) -> Vec<String> {
        let mut recipients: Vec<String> = self
            .messages
            .iter()
            .cloned()
            .map(|msg| msg.recipient.to_string())
            .collect();
        recipients.sort_unstable();
        recipients.dedup();
        recipients
    }

    /// Gets all messages for a given recipient
    pub fn get_messages_for_recipient(&self, recipient: String) -> Vec<AMPMsg> {
        self.messages
            .iter()
            .cloned()
            .filter(|msg| msg.recipient == recipient.clone())
            .collect()
    }

    /// Used to verify that the sender of the AMPPkt is authorised to attach the given origin field.
    /// A sender is valid if:
    ///
    /// 1. The origin matches the sender
    /// 2. The sender is the kernel
    /// 3. The sender has a code ID stored within the ADODB (and as such is a valid ADO)
    ///
    /// If the sender is not valid, an error is returned
    pub fn verify_origin(
        &self,
        sender: &str,
        kernel_address: &str,
        origin: &str,
        deps: Deps,
    ) -> Result<(), ContractError> {
        if sender == origin || sender == kernel_address {
            Ok(())
        } else {
            let adodb_address: Addr =
                deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: kernel_address.to_string(),
                    msg: to_binary(&KernelQueryMsg::KeyAddress {
                        key: ADO_DB_KEY.to_string(),
                    })?,
                }))?;

            // Get the sender's Code ID
            let contract_info: ContractInfoResponse =
                deps.querier
                    .query(&QueryRequest::Wasm(WasmQuery::ContractInfo {
                        contract_addr: sender.to_owned(),
                    }))?;

            let sender_code_id = contract_info.code_id;

            // We query the ADO type in the adodb, it will return an error if the sender's Code ID doesn't exist.
            let verify: Option<String> =
                deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: adodb_address.to_string(),
                    msg: to_binary(&ADODBQueryMsg::ADOType {
                        code_id: sender_code_id,
                    })?,
                }))?;

            if verify.is_some() {
                Ok(())
            } else {
                Err(ContractError::Unauthorized {})
            }
        }
    }

    pub fn get_verified_origin(
        &self,
        sender: &str,
        kernel_address: &str,
        deps: Deps,
    ) -> Result<String, ContractError> {
        let origin = self.get_origin();
        let res = self.verify_origin(sender, kernel_address, origin.as_str(), deps);
        match res {
            Ok(_) => Ok(origin),
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {

    fn extract_chain(s: &str) -> Option<&str> {
        let juno_start = s.find('/')? + 2;
        let juno_end = s[juno_start..].find('/').unwrap_or(s[juno_start..].len());
        Some(&s[juno_start..juno_start + juno_end])
    }

    #[test]
    fn test_explicit_with_protocol() {
        let s = "ibc://juno/path";
        let res = extract_chain(s);
        assert_eq!("juno", res.unwrap())
    }
    #[test]
    fn test_explicit_without_protocol() {
        let s = "juno/path";
        let res = s.split('/').next();
        assert_eq!("juno", res.unwrap())
    }
    #[test]
    fn test_explicit_without_protocol_without_chain() {
        let s = "/path";
        let res = s.split('/').next();
        assert!(res.unwrap().is_empty())
    }
}
