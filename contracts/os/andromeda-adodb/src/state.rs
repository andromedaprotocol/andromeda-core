use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::Map;

/// Stores a mapping between ADO type and its code ID
pub const CODE_ID: Map<&str, u64> = Map::new("code_id");
/// Stores a mapping between a code ID and its type
pub const ADO_TYPE: Map<u64, String> = Map::new("ado_type");

pub fn store_code_id(storage: &mut dyn Storage, code_id_key: &str, code_id: u64) -> StdResult<()> {
    CODE_ID.save(storage, code_id_key, &code_id)?;
    ADO_TYPE.save(storage, code_id, &code_id_key.to_string())
}

pub fn read_code_id(storage: &dyn Storage, code_id_key: &str) -> StdResult<u64> {
    CODE_ID.load(storage, code_id_key)
}
