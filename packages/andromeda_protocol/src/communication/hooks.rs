use cosmwasm_std::{Binary, SubMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::rates::Funds;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AndromedaHook {
    OnExecute {
        sender: String,
        msg: Binary,
    },
    OnFundsTransfer {
        sender: String,
        msg: Binary,
        amount: Funds,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct OnFundsTransferResponse {
    pub msgs: Vec<SubMsg>,
    pub leftover_funds: Funds,
}
