use cosmwasm_std::{to_binary, QuerierWrapper, QueryRequest, StdResult, Storage, WasmQuery};
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const ADDRESS_LIST: Map<String, bool> = Map::new("addresslist");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
pub struct AddressList {
    pub moderators: Vec<String>,
}

impl AddressList {
    pub fn is_moderator(&self, addr: &String) -> bool {
        self.moderators.contains(addr)
    }
    pub fn add_address(&self, storage: &mut dyn Storage, addr: &String) -> StdResult<()> {
        ADDRESS_LIST.save(storage, addr.clone(), &true)
    }
    pub fn remove_address(&self, storage: &mut dyn Storage, addr: &String) -> StdResult<()> {
        ADDRESS_LIST.save(storage, addr.clone(), &false)
    }
    pub fn includes_address(&self, storage: &dyn Storage, addr: &String) -> StdResult<bool> {
        match ADDRESS_LIST.load(storage, addr.clone()) {
            Ok(included) => Ok(included),
            Err(e) => match e {
                cosmwasm_std::StdError::NotFound { .. } => Ok(false),
                _ => Err(e),
            },
        }
    }
}

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
    AddAddress { address: String },
    RemoveAddress { address: String },
    UpdateOwner { address: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    IncludesAddress { address: String },
    ContractOwner {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct IncludesAddressResponse {
    pub included: bool,
}
