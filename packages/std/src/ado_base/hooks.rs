use crate::common::Funds;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Event, SubMsg};

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
    OnTransfer {
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

/// Helper enum for serialization
#[cw_serde]
pub enum HookMsg {
    AndrHook(AndromedaHook),
}
