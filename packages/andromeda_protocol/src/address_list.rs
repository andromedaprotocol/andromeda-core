use cosmwasm_std::{to_binary, QuerierWrapper, QueryRequest, StdResult, Storage, WasmQuery};
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const ADDRESS_LIST: Map<String, bool> = Map::new("addresslist");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
pub struct AddressList {
    /// A list of addresses allowed to add/remove addresses from the list
    pub moderators: Vec<String>,
}

impl AddressList {
    /// Check if an address is a moderator of the address list.
    pub fn is_moderator(&self, addr: &String) -> bool {
        self.moderators.contains(addr)
    }
    /// Add an address to the address list.
    pub fn add_address(&self, storage: &mut dyn Storage, addr: &String) -> StdResult<()> {
        ADDRESS_LIST.save(storage, addr.clone(), &true)
    }
    /// Remove an address from the address list. Errors if the address is not currently included.
    pub fn remove_address(&self, storage: &mut dyn Storage, addr: &String) {
        let included = ADDRESS_LIST.load(storage, addr.clone());

        // Check if the address is included in the address list before removing
        if included.is_ok() {
            ADDRESS_LIST.remove(storage, addr.clone());
        };
    }
    /// Query if a given address is included in the address list.
    pub fn includes_address(&self, storage: &dyn Storage, addr: &String) -> StdResult<bool> {
        match ADDRESS_LIST.load(storage, addr.clone()) {
            Ok(included) => Ok(included),
            Err(e) => match e {
                //If no value for address return false
                cosmwasm_std::StdError::NotFound { .. } => Ok(false),
                _ => Err(e),
            },
        }
    }
}

/// Helper function to query an address list contract for inclusion of an address
///
/// Returns a boolean value indicating whether or not the address is included in the address list
pub fn query_includes_address(
    querier: QuerierWrapper,
    contract_addr: String,
    address: String,
) -> StdResult<bool> {
    let res: IncludesAddressResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr,
        msg: to_binary(&QueryMsg::IncludesAddress { address })?,
    }))?;

    Ok(res.included)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub moderators: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Add an address to the address list
    AddAddress { address: String },
    /// Remove an address from the address list
    RemoveAddress { address: String },
    /// Update ownership of the contract
    UpdateOwner { address: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Query if address is included
    IncludesAddress { address: String },
    /// Query the current contract owner
    ContractOwner {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct IncludesAddressResponse {
    /// Whether the address is included in the address list
    pub included: bool,
}
