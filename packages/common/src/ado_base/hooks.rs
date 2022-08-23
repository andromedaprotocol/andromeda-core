use crate::Funds;
use cosmwasm_std::{Binary, Event, SubMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AndromedaHook {
    OnExecute {
        sender: String,
        payload: Binary,
    },
    OnFundsTransfer {
        sender: String,
        payload: Binary,
        amount: Funds,
    },
    OnTransfer {
        token_id: String,
        sender: String,
        recipient: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct OnFundsTransferResponse {
    pub msgs: Vec<SubMsg>,
    pub events: Vec<Event>,
    pub leftover_funds: Funds,
}

/// Helper enum for serialization
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum HookMsg {
    AndrHook(AndromedaHook),
}
