use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::{Item, Map};

pub const ADDRESS_LIST: Map<&str, bool> = Map::new("addresslist");
pub const IS_INCLUSIVE: Item<bool> = Item::new("is_inclusive");

/// Add an address to the address list.
pub fn add_address(storage: &mut dyn Storage, addr: &str) -> StdResult<()> {
    ADDRESS_LIST.save(storage, addr, &true)
}
/// Remove an address from the address list. Errors if the address is not currently included.
pub fn remove_address(storage: &mut dyn Storage, addr: &str) {
    // Check if the address is included in the address list before removing
    if ADDRESS_LIST.has(storage, addr) {
        ADDRESS_LIST.remove(storage, addr);
    };
}
/// Query if a given address is included in the address list.
pub fn includes_address(storage: &dyn Storage, addr: &str) -> StdResult<bool> {
    match ADDRESS_LIST.load(storage, addr) {
        Ok(included) => Ok(included),
        Err(e) => match e {
            //If no value for address return false
            cosmwasm_std::StdError::NotFound { .. } => Ok(false),
            _ => Err(e),
        },
    }
}
