use andromeda_std::{
    ado_base::permissioning::LocalPermission, amp::AndrAddr, andr_exec, andr_instantiate,
    andr_query,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub actor_permission: Option<ActorPermission>,
}
// Struct used to bundle actor and permission
#[cw_serde]
pub struct ActorPermission {
    pub actors: Vec<AndrAddr>,
    pub permission: LocalPermission,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// Adds an actor key and a permission value
    #[attrs(restricted, nonpayable)]
    PermissionActors {
        actors: Vec<AndrAddr>,
        permission: LocalPermission,
    },
    /// Removes actor alongisde his permission
    #[attrs(restricted, nonpayable)]
    RemovePermissions { actors: Vec<AndrAddr> },
}

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
pub struct IsInclusiveResponse {
    pub is_inclusive_response: bool,
}

#[cw_serde]
pub struct IncludesActorResponse {
    /// Whether the actor is included in the permissions
    pub included: bool,
}

#[cw_serde]
pub struct ActorPermissionResponse {
    pub permission: LocalPermission,
}
