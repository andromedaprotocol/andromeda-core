use crate::error::ContractError;
use cosmwasm_std::{
    to_binary, MessageInfo, QuerierWrapper, QueryRequest, StdResult, Storage, WasmQuery,
};
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const ADDRESS_LIST: Map<String, bool> = Map::new("addresslist");

/// Add an address to the address list.
pub fn add_address(storage: &mut dyn Storage, addr: &str) -> StdResult<()> {
    ADDRESS_LIST.save(storage, addr.to_string(), &true)
}
/// Remove an address from the address list. Errors if the address is not currently included.
pub fn remove_address(storage: &mut dyn Storage, addr: &str) {
    let included = ADDRESS_LIST.load(storage, addr.to_string());

    // Check if the address is included in the address list before removing
    if included.is_ok() {
        ADDRESS_LIST.remove(storage, addr.to_string());
    };
}
/// Query if a given address is included in the address list.
pub fn includes_address(storage: &dyn Storage, addr: &str) -> StdResult<bool> {
    match ADDRESS_LIST.load(storage, addr.to_string()) {
        Ok(included) => Ok(included),
        Err(e) => match e {
            //If no value for address return false
            cosmwasm_std::StdError::NotFound { .. } => Ok(false),
            _ => Err(e),
        },
    }
}

/// Helper function to query an address list contract for inclusion of an address
///
/// Returns a boolean value indicating whether or not the address is included in the address list
pub fn query_includes_address(
    querier: QuerierWrapper,
    contract_addr: String,
    address: String,
) -> Result<bool, ContractError> {
    let res: IncludesAddressResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr,
        msg: to_binary(&QueryMsg::IncludesAddress { address })?,
    }))?;

    Ok(res.included)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub operators: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Add an address to the address list
    AddAddress {
        address: String,
    },
    /// Remove an address from the address list
    RemoveAddress {
        address: String,
    },
    /// Update ownership of the contract
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
    /// Query if address is included
    IncludesAddress {
        address: String,
    },
    /// Query the current contract owner
    ContractOwner {},
    IsOperator {
        address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct IncludesAddressResponse {
    /// Whether the address is included in the address list
    pub included: bool,
}

pub fn on_execute(
    querier: QuerierWrapper,
    addr: String,
    info: MessageInfo,
) -> Result<(), ContractError> {
    query_includes_address(querier, addr, info.sender.to_string())?;
    Ok(())
}
