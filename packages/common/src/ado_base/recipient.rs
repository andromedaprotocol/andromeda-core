use crate::{
    ado_base::{AndromedaMsg, ExecuteMsg},
    encode_binary,
    error::ContractError,
};
use cosmwasm_std::{Api, BankMsg, Binary, Coin, CosmosMsg, SubMsg, WasmMsg};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// ADOs use a default Receive message for handling funds,
/// this struct states that the recipient is an ADO and may attach the data field to the Receive message
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ADORecipient {
    pub addr: String,
    pub msg: Option<Binary>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Recipient {
    Addr(String),
    ADO(ADORecipient),
}

impl Recipient {
    /// Creates an Addr Recipient from the given string
    pub fn from_string(addr: String) -> Recipient {
        Recipient::Addr(addr)
    }

    /// Creates an ADO Recipient from the given string with an empty Data field
    pub fn ado_from_string(addr: String) -> Recipient {
        Recipient::ADO(ADORecipient { addr, msg: None })
    }

    /// Gets the address of the recipient.
    pub fn get_addr(&self) -> String {
        match &self {
            Recipient::Addr(addr) => addr.clone(),
            Recipient::ADO(ado_recipient) => ado_recipient.addr.clone(),
        }
    }
}
