use andromeda_protocol::token::TokenId;
use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenConfig {
    pub name: String,
    pub symbol: String,
    pub minter: String,
}

pub const CONFIG: Item<TokenConfig> = Item::new("config");
pub const OWNERSHIP: Map<String, String> = Map::new("ownership");
