use andromeda_data_storage::primitive::{GetValueResponse, Primitive};
use andromeda_std::error::ContractError;
use cosmwasm_std::Storage;
use cw_storage_plus::Map;

pub const DEFAULT_KEY: &str = "default";

pub const DATA: Map<&str, Primitive> = Map::new("data");

pub fn query_value(
    storage: &dyn Storage,
    key: Option<String>,
) -> Result<GetValueResponse, ContractError> {
    let key = get_key_or_default(&key);
    let value = DATA.load(storage, key)?;
    Ok(GetValueResponse {
        key: key.to_string(),
        value,
    })
}

pub fn get_key_or_default(name: &Option<String>) -> &str {
    match name {
        None => DEFAULT_KEY,
        Some(s) => s,
    }
}
