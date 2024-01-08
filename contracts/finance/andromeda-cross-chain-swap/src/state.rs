use andromeda_std::amp::{messages::AMPCtx, AndrAddr};
use cosmwasm_std::Binary;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
pub struct ForwardReplyState {
    pub addr: AndrAddr,
    pub msg: Option<Binary>,
    pub dex: String,
    pub amp_ctx: Option<AMPCtx>,
}

pub const FORWARD_REPLY_STATE: Item<ForwardReplyState> = Item::new("forward_reply_state");
