use crate::modules::ModuleDefinition;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub token_code_id: u64,
    pub address_list_code_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    //Create new token
    Create {
        name: String,
        symbol: String,
        modules: Vec<ModuleDefinition>,
        metadata_limit: Option<u64>,
    },
    //Called by instantiated token contract to store address
    TokenCreationHook {
        symbol: String,
        creator: String,
    },
    UpdateAddress {
        symbol: String,
        new_address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetAddress { symbol: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AddressResponse {
    pub address: String,
}
