use andromeda_protocol::token::TOKEN_ID;
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
pub const OWNERSHIP: Map<TOKEN_ID, String> = Map::new("ownership");

pub fn store_config(storage: &mut dyn Storage, config: &TokenConfig) -> StdResult<TokenConfig> {
    CONFIG.save(storage, config)?;
    Ok(config.clone())
}

pub fn store_owner(
    storage: &mut dyn Storage,
    token_id: &TOKEN_ID,
    owner: &String,
) -> StdResult<()> {
    OWNERSHIP.save(storage, token_id, owner)
}

pub fn get_owner(storage: &mut dyn Storage, token_id: &TOKEN_ID) -> StdResult<String> {
    OWNERSHIP.may_load(storage, token_id)
}
