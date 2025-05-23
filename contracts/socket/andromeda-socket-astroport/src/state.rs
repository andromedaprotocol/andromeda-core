use andromeda_std::{
    amp::{messages::AMPCtx, AndrAddr, Recipient},
    common::denom::Asset,
};
use cosmwasm_std::Uint128;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
pub struct ForwardReplyState {
    /// Forward Recipient
    pub recipient: Recipient,
    /// Refund Address
    pub refund_addr: AndrAddr,
    /// Amp ctx to be used for ibc communication
    pub amp_ctx: Option<AMPCtx>,
    /// Offered asset to the astroport
    pub from_asset: Asset,
    /// Asked asset returning from the astroport
    pub to_asset: Asset,
}

pub const FORWARD_REPLY_STATE: Item<ForwardReplyState> = Item::new("forward_reply_state");

pub const SWAP_ROUTER: Item<AndrAddr> = Item::new("swap_router");

pub const FACTORY: Item<AndrAddr> = Item::new("factory");

pub const PREV_BALANCE: Item<Uint128> = Item::new("prev_balance");
