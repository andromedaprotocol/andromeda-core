pub mod ado_type;
pub mod block_height;
pub mod hooks;
pub mod modules;
pub mod operators;
pub mod ownership;
pub mod recipient;
pub mod version;

use crate::{
    ado_base::{modules::Module, recipient::Recipient},
    error::ContractError,
    withdraw::Withdrawal,
};
use cosmwasm_std::{to_binary, Binary, QuerierWrapper, QueryRequest, Uint64, WasmQuery};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InstantiateMsg {
    pub ado_type: String,
    pub ado_version: String,
    pub operators: Option<Vec<String>>,
    pub modules: Option<Vec<Module>>,
    pub primitive_contract: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AndromedaMsg {
    /// Standard Messages
    Receive(Option<Binary>),
    UpdateOwner {
        address: String,
    },
    UpdateOperators {
        operators: Vec<String>,
    },
    UpdateAppContract {
        address: String,
    },
    UpdateVersion {
        version: String,
    },
    Withdraw {
        recipient: Option<Recipient>,
        tokens_to_withdraw: Option<Vec<Withdrawal>>,
    },
    RegisterModule {
        module: Module,
    },
    DeregisterModule {
        module_idx: Uint64,
    },
    AlterModule {
        module_idx: Uint64,
        module: Module,
    },
    RefreshAddress {
        contract: String,
    },
    RefreshAddresses {
        limit: Option<u32>,
        start_after: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AndromedaQuery {
    Get(Option<Binary>),
    Owner {},
    Operators {},
    Type {},
    OriginalPublisher {},
    BlockHeightUponCreation {},
    IsOperator { address: String },
    Module { id: Uint64 },
    ModuleIds {},
    Version {},
}

/// Helper enum for serialization
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
}

/// Helper enum for serialization
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
}

/// Helper function for querying a contract using AndromedaQuery::Get
pub fn query_get<T>(
    data: Option<Binary>,
    address: String,
    querier: &QuerierWrapper,
) -> Result<T, ContractError>
where
    T: DeserializeOwned,
{
    let query_msg = QueryMsg::AndrQuery(AndromedaQuery::Get(data));
    let resp: T = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: address,
        msg: to_binary(&query_msg)?,
    }))?;

    Ok(resp)
}
