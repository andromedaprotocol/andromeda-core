use andromeda_std::{ado_base::permissioning::Permission, andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub is_inclusive: bool,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// Add an address to the address list
    AddAddress { address: String },
    /// Remove an address from the address list
    RemoveAddress { address: String },
    /// Add multiple addresses to the address list
    AddAddresses { addresses: Vec<String> },
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
    /// Query if address is included
    #[returns(IncludesAddressResponse)]
    IncludesAddress { address: String },
    #[returns(bool)]
    IsInclusive {},
    #[returns(IncludesActorResponse)]
    IncludesActor { actor: Addr },
    #[returns(ActorPermissionResponse)]
    ActorPermission { actor: Addr },
}

#[cw_serde]
pub struct IncludesAddressResponse {
    /// Whether the address is included in the address list
    pub included: bool,
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
