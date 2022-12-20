use common::ado_base::{hooks::AndromedaHook, AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
pub struct InstantiateMsg {
    pub is_inclusive: bool,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    /// Add an address to the address list
    AddAddress {
        address: String,
    },
    /// Remove an address from the address list
    RemoveAddress {
        address: String,
    },
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Query if address is included
    #[returns(IncludesAddressResponse)]
    IncludesAddress { address: String },
    /// Query the current contract owner
    #[returns(AndromedaHook)]
    AndrHook(AndromedaHook),
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
}

#[cw_serde]
pub struct IncludesAddressResponse {
    /// Whether the address is included in the address list
    pub included: bool,
}
