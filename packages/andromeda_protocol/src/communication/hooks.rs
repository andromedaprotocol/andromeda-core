use crate::rates::Funds;
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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct OnFundsTransferResponse {
    pub msgs: Vec<SubMsg>,
    pub events: Vec<Event>,
    pub leftover_funds: Funds,
}
