use andromeda_std::ado_base::permissioning::Permission;
use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::{Item, Map};

pub const ADDRESS_LIST: Map<&str, bool> = Map::new("addresslist");
pub const IS_INCLUSIVE: Item<bool> = Item::new("is_inclusive");
/// A mapping of actor to permission
pub const PERMISSIONS: Map<&Addr, Permission> = Map::new("permissioning");

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
    Ok(ADDRESS_LIST.may_load(storage, addr)?.unwrap_or(false))
}
/// Query if a given actor is included in the permissions list.
pub fn includes_actor(storage: &dyn Storage, actor: &Addr) -> StdResult<bool> {
    Ok(PERMISSIONS.has(storage, actor))
}

/// Add or update an actor's permission
pub fn add_actor_permission(
    storage: &mut dyn Storage,
    actor: &Addr,
    permission: &Permission,
) -> StdResult<()> {
    PERMISSIONS.save(storage, actor, permission)
}
