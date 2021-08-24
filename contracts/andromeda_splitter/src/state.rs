use cosmwasm_std::{Addr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw_storage_plus::Item;
// use std::collections::HashMap; 
// use andromeda_protocol::modules::whitelist::Whitelist;
use andromeda_protocol::token::TokenId;


pub const STATE: Item<State> = Item::new("state");
pub const SPLITTER: Item<Splitter> = Item::new("splitter");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: Addr,              // creator address    
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AddressPercent{
    pub addr: String,
    pub percent: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Splitter{
    // pub recipient: HashMap<String, u16>,   //Map for Address and Percentage
    pub recipient: Vec<AddressPercent>,   //Map for Address and Percentage
    pub is_lock: bool,                     //Lock
    pub sender_whitelist: Vec<String>,       //Address List allowing to receive funds
    pub accepted_tokenlist: Vec<TokenId>,  //Token List allowing to accept
}