use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw_storage_plus::Item;
use andromeda_protocol::modules::whitelist::Whitelist;
use andromeda_protocol::token::TokenId;


pub const STATE: Item<State> = Item::new("state");
pub const SPLITTER: Item<Splitter> = Item::new("splitter");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: Addr,              // owner address
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AddressPercent{
    pub addr: String,
    pub percent: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Splitter{
    pub recipient: Vec<AddressPercent>,   //Map for address and percentage
    pub is_lock: bool,                     //Lock
    pub use_whitelist: bool,               //Use whitelist
    pub sender_whitelist: Whitelist,       //Address list allowing to receive funds
    pub accepted_tokenlist: Vec<TokenId>,  //Token list allowing to accept
}