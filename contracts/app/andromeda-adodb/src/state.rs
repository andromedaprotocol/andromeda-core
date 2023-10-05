use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::Map;

pub const CODE_ID: Map<&str, u64> = Map::new("code_id");

pub fn store_code_id(storage: &mut dyn Storage, code_id_key: &str, code_id: u64) -> StdResult<()> {
    CODE_ID.save(storage, code_id_key, &code_id)
}

pub fn read_code_id(storage: &dyn Storage, code_id_key: &str) -> StdResult<u64> {
    CODE_ID.load(storage, code_id_key)
}
