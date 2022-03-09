use crate::{
    ado_base::{hooks::AndromedaHook, AndromedaMsg, AndromedaQuery},
    error::ContractError,
};
use cosmwasm_std::{to_binary, CosmosMsg, Event, SubMsg, Uint128, WasmMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// The address authorized to mint new receipts
    pub minter: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
/// A struct representation of a receipt. Contains a vector of CosmWasm [Event](https://docs.rs/cosmwasm-std/0.16.0/cosmwasm_std/struct.Event.html) structs.
pub struct Receipt {
    /// A vector of CosmWasm [Event](https://docs.rs/cosmwasm-std/0.16.0/cosmwasm_std/struct.Event.html) structs related to the receipt
    pub events: Vec<Event>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// The address authorized to mint new receipts
    pub minter: String,
    /// Optional list of moderating addresses authorized to update receipts, defaults to an empty vector
    pub operators: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    /// Mint a new receipt. Only executable by the assigned `minter` address. Generates a receipt ID.
    StoreReceipt {
        receipt: Receipt,
    },
    /// Edit a receipt by ID. Only executable by the assigned `minter` address or a valid `operator`.
    EditReceipt {
        receipt_id: Uint128,
        receipt: Receipt,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    /// Query receipt by its generated ID.
    Receipt {
        receipt_id: Uint128,
    },
    /// The current contract config.
    ContractInfo {},
    AndrHook(AndromedaHook),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ContractInfoResponse {
    pub config: Config,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ReceiptResponse {
    pub receipt: Receipt,
}

pub fn generate_receipt_message(
    contract_addr: String,
    events: Vec<Event>,
) -> Result<SubMsg, ContractError> {
    let receipt = Receipt { events };

    Ok(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr,
        msg: to_binary(&ExecuteMsg::StoreReceipt { receipt })?,
        funds: vec![],
    })))
}
