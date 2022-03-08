use crate::{
    communication::{query_get, AndromedaMsg, AndromedaQuery},
    modules::ModuleDefinition,
    primitive::{get_address, AndromedaContract},
    ContractError,
};
use cosmwasm_std::{to_binary, QuerierWrapper, Storage};
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

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

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

pub fn get_ado_codeid(
    storage: &dyn Storage,
    querier: &QuerierWrapper,
    name: &str,
) -> Result<Option<u64>, ContractError> {
    let factory_address = get_address(storage, querier, AndromedaContract::Factory)?;

    let code_id_resp: CodeIdResponse =
        query_get(Some(to_binary(name)?), factory_address, &querier)?;
    Ok(Some(code_id_resp.code_id))
}
