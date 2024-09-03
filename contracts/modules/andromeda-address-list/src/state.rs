use andromeda_std::ado_base::permissioning::LocalPermission;
use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::Map;

/// A mapping of actor to LocalPermission. Contract Permission is not supported in this contract
pub const PERMISSIONS: Map<&Addr, LocalPermission> = Map::new("permissioning");

/// Query if a given actor is included in the permissions list.
pub fn includes_actor(storage: &dyn Storage, actor: &Addr) -> StdResult<bool> {
    Ok(PERMISSIONS.has(storage, actor))
}

/// Add or update an actor's permission
pub fn add_actors_permission(
    storage: &mut dyn Storage,
    actors: Vec<Addr>,
    permission: &LocalPermission,
) -> StdResult<()> {
    for actor in actors {
        PERMISSIONS.save(storage, &actor, permission)?;
    }
    Ok(())
}
