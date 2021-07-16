use cosmwasm_std::{HumanAddr, StdResult, Storage};
use cosmwasm_storage::{bucket, bucket_read, singleton};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenConfig {
    pub name: String,
    pub symbol: String,
}

static CONFIG_KEY: &[u8] = b"config";
static OWNERS_NS: &[u8] = b"owners";

pub fn store_config<S: Storage>(storage: &mut S, config: &TokenConfig) -> StdResult<()> {
    singleton(storage, CONFIG_KEY).save(config)
}

pub fn store_owner<S: Storage>(
    storage: &mut S,
    token_id: &i64,
    owner: &HumanAddr,
) -> StdResult<()> {
    bucket(OWNERS_NS, storage).save(&token_id.to_le_bytes(), owner)
}

pub fn get_owner<S: Storage>(storage: &S, token_id: &i64) -> StdResult<HumanAddr> {
    bucket_read(OWNERS_NS, storage).load(&token_id.to_le_bytes())
}
