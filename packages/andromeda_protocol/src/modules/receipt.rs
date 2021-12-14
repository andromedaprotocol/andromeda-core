use cosmwasm_std::{
    to_binary, CosmosMsg, DepsMut, Env, Event, MessageInfo, Reply, ReplyOn, Response, StdError,
    StdResult, Storage, SubMsg, WasmMsg,
};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::receipt::Receipt;
use crate::response::get_reply_address;
use crate::{
    modules::{
        common::is_unique,
        hooks::{HookResponse, MessageHooks},
        read_modules, {Module, ModuleDefinition},
    },
    receipt::{ExecuteMsg, InstantiateMsg},
    require,
};
pub const RECEIPT_CONTRACT: Item<String> = Item::new("receiptcontract");
pub const REPLY_RECEIPT: u64 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// A struct used to define the Receipt module. Can be defined by providing either a contract address or the combination of a code ID and a vector of moderators.
pub struct ReceiptModule {
    /// The address of the module contract
    pub address: Option<String>,
    /// The code ID for the module contract
    pub code_id: Option<u64>,
    /// An optional vector of addresses to assign as moderators
    pub moderators: Option<Vec<String>>,
}

impl ReceiptModule {
    /// Creates a `CosmosMsg::Wasm` message to mint a receipt on the module contract
    /// Errors if the receipt module does not have an assigned contract address.
    pub fn generate_receipt_message(
        self,
        storage: &dyn Storage,
        events: Vec<Event>,
    ) -> StdResult<CosmosMsg> {
        let receipt = Receipt { events };

        let contract_addr = self
            .get_contract_address(storage)
            // [REC-01] Replace ok_or with lazily ok_or_else to optimizr smart contract efficiency
            .ok_or_else(|| {
                StdError::generic_err("Receipt module does not have an assigned address")
            })?;

        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg: to_binary(&ExecuteMsg::StoreReceipt { receipt })?,
            funds: vec![],
        }))
    }
}

impl Module for ReceiptModule {
    /// Validates the receipt module:
    /// * Must be unique
    /// * Must include either a contract address or a combination of a valid code id and an optional vector of moderating addresses
    fn validate(&self, all_modules: Vec<ModuleDefinition>) -> StdResult<bool> {
        require(
            is_unique(self, &all_modules),
            StdError::generic_err("The receipt module must be unique"),
        )?;

        require(
            self.address.is_some() || (self.code_id.is_some() && self.moderators.is_some()),
            StdError::generic_err(
                "Receipt must include either a contract address or a code id and moderator list",
            ),
        )?;

        Ok(true)
    }
    fn as_definition(&self) -> ModuleDefinition {
        ModuleDefinition::Receipt {
            address: self.address.clone(),
            code_id: self.code_id.clone(),
            moderators: self.moderators.clone(),
        }
    }
    fn get_contract_address(&self, storage: &dyn Storage) -> Option<String> {
        if self.address.clone().is_some() {
            return Some(self.address.clone().unwrap());
        }
        RECEIPT_CONTRACT.may_load(storage).unwrap()
    }
}

impl MessageHooks for ReceiptModule {
    /// Creates a `SubMsg` with which to instantiate the receipt module contract
    fn on_instantiate(
        &self,
        _deps: &DepsMut,
        info: MessageInfo,
        _env: Env,
    ) -> StdResult<HookResponse> {
        let mut res = HookResponse::default();
        if self.address.is_none() {
            let inst_msg = WasmMsg::Instantiate {
                admin: Some(info.sender.to_string()),
                code_id: self.code_id.unwrap(),
                funds: vec![],
                label: String::from("Receipt instantiation"),
                msg: to_binary(&InstantiateMsg {
                    minter: info.sender.to_string(),
                    moderators: self.moderators.clone(),
                })?,
            };

            let msg = SubMsg {
                msg: inst_msg.into(),
                gas_limit: None,
                id: REPLY_RECEIPT,
                reply_on: ReplyOn::Always,
            };

            res = res.add_message(msg);
        }

        Ok(res)
    }
}

pub fn on_receipt_reply(deps: DepsMut, msg: Reply) -> StdResult<Response> {
    let receipt_addr = get_reply_address(msg)?;

    RECEIPT_CONTRACT.save(deps.storage, &receipt_addr.to_string())?;

    Ok(Response::new())
}

/// Searches the stored vector of Modules within the current contract for a receipt module
pub fn get_receipt_module(storage: &dyn Storage) -> StdResult<Option<ReceiptModule>> {
    let modules = read_modules(storage)?;
    let receipt_def = modules.module_defs.iter().find(|m| match m {
        ModuleDefinition::Receipt { .. } => true,
        _ => false,
    });

    if receipt_def.is_none() {
        return Ok(None);
    }

    match receipt_def.unwrap() {
        ModuleDefinition::Receipt {
            moderators,
            code_id,
            address,
        } => Ok(Some(ReceiptModule {
            moderators: moderators.clone(),
            code_id: *code_id,
            address: address.clone(),
        })),
        _ => Ok(None),
    }
}
