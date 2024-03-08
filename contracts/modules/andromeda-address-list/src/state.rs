use andromeda_std::ado_base::permissioning::Permission;
use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::Map;

/// A mapping of actor to permission
pub const PERMISSIONS: Map<&Addr, Permission> = Map::new("permissioning");

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
