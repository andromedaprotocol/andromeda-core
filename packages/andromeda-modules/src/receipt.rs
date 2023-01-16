use common::{
    ado_base::{hooks::AndromedaHook, AndromedaMsg, AndromedaQuery},
    error::ContractError,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_binary, CosmosMsg, Event, SubMsg, Uint128, WasmMsg};

#[cw_serde]
pub struct Config {
    /// The address authorized to mint new receipts
    pub minter: String,
}

#[cw_serde]
/// A struct representation of a receipt. Contains a vector of CosmWasm
/// [Event](https://docs.rs/cosmwasm-std/0.16.0/cosmwasm_std/struct.Event.html) structs.
pub struct Receipt {
    /// A vector of CosmWasm [Event](https://docs.rs/cosmwasm-std/0.16.0/cosmwasm_std/struct.Event.html) structs related to the receipt
    pub events: Vec<Event>,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// The address authorized to mint new receipts
    pub minter: String,
    pub kernel_address: Option<String>,
}

#[cw_serde]
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

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    /// Query receipt by its generated ID.
    #[returns(ReceiptResponse)]
    Receipt { receipt_id: Uint128 },
    /// The current contract config.
    #[returns(ContractInfoResponse)]
    ContractInfo {},
    #[returns(AndromedaHook)]
    AndrHook(AndromedaHook),
}

#[cw_serde]
pub struct ContractInfoResponse {
    pub config: Config,
}

#[cw_serde]
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
