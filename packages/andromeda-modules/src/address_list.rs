use andromeda_std::{ado_base::permissioning::Permission, andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub actor: Addr,
    pub permission: Permission,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// Adds an actor key and a permission value
    AddActorPermission { actor: Addr, permission: Permission },
    /// Removes actor alongisde his permission
    RemoveActorPermission { actor: Addr },
}

#[cw_serde]
pub struct MigrateMsg {}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(IncludesActorResponse)]
    IncludesActor { actor: Addr },
    #[returns(ActorPermissionResponse)]
    ActorPermission { actor: Addr },
}

#[cw_serde]
pub struct IncludesActorResponse {
    /// Whether the actor is included in the permissions
    pub included: bool,
}

#[cw_serde]
pub struct ActorPermissionResponse {
    pub permission: Permission,
}
