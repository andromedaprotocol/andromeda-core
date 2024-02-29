use crate::common::Funds;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Event, SubMsg};

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
