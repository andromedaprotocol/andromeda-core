use crate::state::{DATA, DEFAULT_KEY, KEY_OWNER, RESTRICTION};
use andromeda_data_storage::primitive::{GetTypeResponse, GetValueResponse, PrimitiveRestriction};
use andromeda_std::{ado_contract::ADOContract, amp::AndrAddr, error::ContractError};
use cosmwasm_std::{Addr, Deps, Storage};

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

pub fn all_keys(storage: &dyn Storage) -> Result<Vec<String>, ContractError> {
    let keys = DATA
        .keys(storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|key| key.unwrap())
        .collect();
    Ok(keys)
}

pub fn owner_keys(deps: &Deps, owner: AndrAddr) -> Result<Vec<String>, ContractError> {
    let owner = owner.get_raw_address(deps)?;
    let keys = KEY_OWNER
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter(|x| x.as_ref().unwrap().1 == owner)
        .map(|key| key.unwrap().0)
        .collect();
    Ok(keys)
}

pub fn get_value(
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

pub fn get_type(
    storage: &dyn Storage,
    key: Option<String>,
) -> Result<GetTypeResponse, ContractError> {
    let key = get_key_or_default(&key);
    let value_type = DATA.load(storage, key)?.from_string();
    Ok(GetTypeResponse { value_type })
}
