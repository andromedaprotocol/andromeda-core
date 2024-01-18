use crate::common::Funds;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Coin, Event, SubMsg};

#[cw_serde]
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
    OnTokenTransfer {
        token_id: String,
        sender: String,
        recipient: String,
    },
}

#[cw_serde]
pub struct OnFundsTransferResponse {
    pub msgs: Vec<SubMsg>,
    pub events: Vec<Event>,
    pub leftover_funds: Funds,
}

impl Default for OnFundsTransferResponse {
    fn default() -> Self {
        Self {
            msgs: Vec::new(),
            events: Vec::new(),
            leftover_funds: Funds::Native(Coin::default()),
        }
    }
}

/// Helper enum for serialization
#[cw_serde]
pub enum HookMsg {
    AndrHook(AndromedaHook),
}
