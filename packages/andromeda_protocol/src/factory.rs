use crate::modules::ModuleDefinition;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub token_code_id: u64,
    pub receipt_code_id: u64,
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
    },
    UpdateAddress {
        symbol: String,
        new_address: String,
    },
    UpdateCodeId {
        receipt_code_id: Option<u64>,
        address_list_code_id: Option<u64>,
        token_code_id: Option<u64>,
    },
    UpdateOwner {
        address: String,
    },
    UpdateOperator {
        operators: Vec<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetAddress { symbol: String },
    CodeIds {},
    ContractOwner {},
    IsOperator { address: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AddressResponse {
    pub address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CodeIdsResponse {
    pub receipt_code_id: u64,
    pub token_code_id: u64,
    pub address_list_code_id: u64,
}
