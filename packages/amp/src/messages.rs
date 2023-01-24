use common::error::ContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, Api, Binary, Coin, CosmosMsg, QuerierWrapper, ReplyOn, SubMsg, WasmMsg,
};

#[cw_serde]
pub enum ExecuteMsg {
    AMPReceive(AMPPkt),
}

#[cw_serde]
pub enum VFSQueryMsg {
    ResolvePath { path: String },
}

#[cw_serde]
/// This struct defines how the kernel parses and relays messages between ADOs
/// It contains a simple recipient string which may use our namespacing implementation or a simple contract address
/// If the desired recipient is via IBC then namespacing must be employed
/// The attached message must be a binary encoded execute message for the receiving ADO
/// Funds can be attached for an individual message and will be attached accordingly
pub struct AMPMsg {
    /// The message recipient, can be a contract/wallet address or a namespaced URI
    pub recipient: String,
    /// The message to be sent to the recipient
    pub message: Binary,
    /// Any funds to be attached to the message, defaults to an empty vector
    pub funds: Vec<Coin>,
    /// When the message should reply, defaults to Always
    pub reply_on: ReplyOn,
    /// An optional imposed gas limit for the message
    pub gas_limit: Option<u64>,
}

impl AMPMsg {
    /// Creates a new AMPMsg
    pub fn new(
        recipient: impl Into<String>,
        message: Binary,
        funds: Option<Vec<Coin>>,
        reply_on: Option<ReplyOn>,
        gas_limit: Option<u64>,
    ) -> AMPMsg {
        AMPMsg {
            recipient: recipient.into(),
            message,
            funds: funds.unwrap_or_default(),
            reply_on: reply_on.unwrap_or(ReplyOn::Always),
            gas_limit,
        }
    }

    /// Gets the address for the recipient
    pub fn get_recipient_address(
        &self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        vfs_contract: Option<Addr>,
    ) -> Result<Addr, ContractError> {
        if self.recipient.contains('/') {
            match vfs_contract {
                Some(vfs_contract) => {
                    let query = VFSQueryMsg::ResolvePath {
                        path: self.recipient.clone(),
                    };
                    return Ok(querier.query_wasm_smart(vfs_contract, &query)?);
                }
                None => return Err(ContractError::InvalidAddress {}),
            }
        }

        let addr = api.addr_validate(&self.recipient);
        match addr {
            Ok(addr) => Ok(addr),
            Err(_) => Err(ContractError::InvalidAddress {}),
        }
    }

    /// Generates a sub message for the given AMP Message
    pub fn generate_sub_message(
        &self,
        contract_addr: impl Into<String>,
        origin: String,
        previous_sender: String,
        id: u64,
    ) -> Result<SubMsg, ContractError> {
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
    Kernel(ReplyGas),
}

#[cw_serde]
pub struct ReplyGas {
    pub reply_on: Option<ReplyOn>,
    pub gas_limit: Option<u64>,
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
            .map(|msg| msg.recipient)
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
}
