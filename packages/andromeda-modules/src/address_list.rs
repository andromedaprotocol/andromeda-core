use common::ado_base::{hooks::AndromedaHook, AndromedaMsg, AndromedaQuery};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub is_inclusive: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
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

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Query if address is included
    IncludesAddress {
        address: String,
    },
    /// Query the current contract owner
    AndrHook(AndromedaHook),
    AndrQuery(AndromedaQuery),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IncludesAddressResponse {
    /// Whether the address is included in the address list
    pub included: bool,
}
