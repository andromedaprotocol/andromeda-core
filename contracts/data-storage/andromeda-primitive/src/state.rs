use andromeda_data_storage::primitive::{GetValueResponse, Primitive, PrimitiveRestriction};
use andromeda_std::{ado_contract::ADOContract, error::ContractError};
use cosmwasm_std::{Addr, Storage};
use cw_storage_plus::{Item, Map};

pub const DEFAULT_KEY: &str = "default";

pub const DATA: Map<&str, Primitive> = Map::new("data");
pub const KEY_OWNER: Map<&str, Addr> = Map::new("key_owner");
pub const RESTRICTION: Item<PrimitiveRestriction> = Item::new("restriction");

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

pub fn has_key_permission(
    storage: &dyn Storage,
    addr: &Addr,
    key: &str,
) -> Result<bool, ContractError> {
    let is_operator = ADOContract::default().is_owner_or_operator(storage, addr.as_str())?;
    let allowed = match RESTRICTION.load(storage)? {
        PrimitiveRestriction::Private => is_operator,
        PrimitiveRestriction::Public => true,
        PrimitiveRestriction::Restricted => match KEY_OWNER.load(storage, key).ok() {
            Some(owner) => addr == owner,
            None => true,
        },
    };
    Ok(is_operator || allowed)
}
