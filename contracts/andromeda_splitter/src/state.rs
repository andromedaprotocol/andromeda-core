use andromeda_protocol::splitter::Splitter;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const STATE: Item<State> = Item::new("state");
pub const SPLITTER: Item<Splitter> = Item::new("splitter");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: Addr, // owner address
}
