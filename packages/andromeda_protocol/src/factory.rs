use crate::{
    communication::{AndromedaMsg, AndromedaQuery},
    modules::ModuleDefinition,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    /// Create new token
    Create {
        name: String,
        symbol: String,
        modules: Vec<ModuleDefinition>,
    },
    UpdateCodeId {
        code_id_key: String,
        code_id: u64,
    },
    /// Update token contract address by symbol
    UpdateAddress {
        symbol: String,
        new_address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    /// Query token contract address by its symbol
    GetAddress {
        symbol: String,
    },
    /// All code IDs for Andromeda contracts
    CodeId {
        key: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AddressResponse {
    pub address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CodeIdResponse {
    pub code_id: u64,
}
