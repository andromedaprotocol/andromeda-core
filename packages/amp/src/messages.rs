use common::error::ContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Binary, Coin, CosmosMsg, QuerierWrapper, ReplyOn, SubMsg, WasmMsg};

#[cw_serde]
/// This struct defines how the kernel parses and relays messages between ADOs
/// It contains a simple recipient string which may use our namespacing implementation or a simple contract address
/// If the desired recipient is via IBC then namespacing must be employeed
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
    /// Creates a new AMPMessage
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
            funds: funds.unwrap_or(vec![]),
            reply_on: reply_on.unwrap_or(ReplyOn::Always),
            gas_limit,
        }
    }

    /// Gets the address for the recipient
    pub fn get_recipient_address(
        &self,
        api: &dyn Api,
        _querier: &QuerierWrapper,
        namespacing_contract: Option<Addr>,
    ) -> Result<String, ContractError> {
        let addr = api.addr_validate(&self.recipient);
        match addr {
            Ok(addr) => Ok(addr.to_string()),
            Err(_) => match namespacing_contract {
                // Some(namespacing_contract) => query_get::<String>(
                //     Some(encode_binary(&self.identifier)?),
                //     app_contract.to_string(),
                //     querier,
                // ),
                // TODO: Add Namespacing here, will need to include IBC
                _ => Err(ContractError::InvalidAddress {}),
            },
        }
    }

    /// Generates a sub message for the given AMP Message
    pub fn generate_message(
        &self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        namespacing_contract: Option<Addr>,
        id: u64,
    ) -> Result<SubMsg, ContractError> {
        Ok(SubMsg {
            id,
            reply_on: self.reply_on.clone(),
            gas_limit: self.gas_limit,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: self.get_recipient_address(api, querier, namespacing_contract)?,
                msg: self.message.clone(),
                funds: self.funds.to_vec(),
            }),
        })
    }
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
    pub fn get_origin(self) -> String {
        self.origin
    }

    /// Gets the previous sender of a message
    pub fn get_previous_sender(self) -> String {
        self.previous_sender
    }
}
