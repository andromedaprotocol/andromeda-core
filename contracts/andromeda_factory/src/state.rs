use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONFIG: Item<Config> = Item::new("config");
pub const SYM_ADDRESS: Map<String, String> = Map::new("address");
pub const SYM_CREATOR: Map<String, String> = Map::new("creator");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: String,
    pub token_code_id: u64,
    pub address_list_code_id: u64,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    CONFIG.save(storage, config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}

pub fn store_address(storage: &mut dyn Storage, symbol: String, address: &String) -> StdResult<()> {
    SYM_ADDRESS.save(storage, symbol, &address)
}

pub fn read_address(storage: &dyn Storage, symbol: String) -> StdResult<String> {
    SYM_ADDRESS.load(storage, symbol)
}

pub fn store_creator(storage: &mut dyn Storage, symbol: String, creator: &String) -> StdResult<()> {
    SYM_CREATOR.save(storage, symbol, creator)
}

pub fn read_creator(storage: &dyn Storage, symbol: String) -> StdResult<String> {
    SYM_CREATOR.load(storage, symbol)
}

pub fn is_address_defined(storage: &dyn Storage, symbol: String) -> StdResult<bool> {
    match read_address(storage, symbol) {
        Ok(_addr) => Ok(true),
        _ => Ok(false),
    }
}

pub fn is_creator(storage: &dyn Storage, symbol: String, address: String) -> StdResult<bool> {
    match read_creator(storage, symbol) {
        Ok(creator) => Ok(address == creator),
        Err(_e) => Ok(false),
    }
}
