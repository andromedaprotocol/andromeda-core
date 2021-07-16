use cosmwasm_std::{CanonicalAddr, HumanAddr, StdResult, Storage};
use cosmwasm_storage::{bucket, bucket_read, singleton, singleton_read};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

static KEY_CONFIG: &[u8] = b"config";
static KEY_ADDRESS: &[u8] = b"address";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub token_code_id: u64,
}

pub fn store_config<S: Storage>(storage: &mut S, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config<S: Storage>(storage: &S) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

pub fn store_address<S: Storage>(
    storage: &mut S,
    symbol: String,
    address: HumanAddr,
) -> StdResult<()> {
    bucket(KEY_ADDRESS, storage).save(symbol.as_bytes(), &address)
}

pub fn read_address<S: Storage>(storage: &S, symbol: String) -> StdResult<HumanAddr> {
    match bucket_read(KEY_ADDRESS, storage).load(symbol.as_bytes()) {
        Ok(addr) => Ok(addr),
        Err(e) => Err(e),
    }
}

pub fn is_address_defined<S: Storage>(storage: &S, symbol: String) -> StdResult<bool> {
    match read_address(storage, symbol) {
        Ok(_addr) => Ok(true),
        _ => Ok(false),
    }
}
