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

pub fn store_config(storage: &mut dyn Storage, config: &TokenConfig) -> StdResult<TokenConfig> {
    CONFIG.save(storage, config)?;
    Ok(config.clone())
}

pub fn store_owner(storage: &mut dyn Storage, token_id: &TokenId, owner: &String) -> StdResult<()> {
    OWNERSHIP.save(storage, token_id.to_string(), owner)
}

pub fn get_owner(storage: &dyn Storage, token_id: &TokenId) -> StdResult<String> {
    OWNERSHIP.load(storage, token_id.to_string())
}
