use crate::modules::ModuleDefinition;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Token Contract Code ID
    pub token_code_id: u64,
    /// Receipt Contract Code ID
    pub receipt_code_id: u64,
    /// Address List Contract Code ID
    pub address_list_code_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Create new token
    Create {
        name: String,
        symbol: String,
        modules: Vec<ModuleDefinition>,
    },
    /// Update token contract address by symbol
    UpdateAddress { symbol: String, new_address: String },
    /// Update code ID for Andromeda contracts
    UpdateCodeId {
        receipt_code_id: Option<u64>,
        address_list_code_id: Option<u64>,
        token_code_id: Option<u64>,
    },
    /// Update current contract owner
    UpdateOwner { address: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Query token contract address by its symbol
    GetAddress { symbol: String },
    /// All code IDs for Andromeda contracts
    CodeIds {},
    /// The current contract owner
    ContractOwner {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AddressResponse {
    pub address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CodeIdsResponse {
    /// Token Contract Code ID
    pub token_code_id: u64,
    /// Receipt Contract Code ID
    pub receipt_code_id: u64,
    /// Address List Contract Code ID
    pub address_list_code_id: u64,
}
