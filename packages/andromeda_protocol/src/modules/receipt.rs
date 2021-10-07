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
    require::require,
};
pub const RECEIPT_CONTRACT: Item<String> = Item::new("receiptcontract");
pub const REPLY_RECEIPT: u64 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ReceiptModule {
    pub address: Option<String>,
    pub code_id: Option<u64>,
    pub moderators: Option<Vec<String>>,
}

impl ReceiptModule {
    pub fn generate_receipt_message(
        self,
        storage: &dyn Storage,
        events: Vec<Event>,
    ) -> StdResult<CosmosMsg> {
        let receipt = Receipt { events };

        let contract_addr = self
            .get_contract_address(storage)
            .ok_or(StdError::generic_err(
                "Receipt module does not have an assigned address",
            ))?;

        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg: to_binary(&ExecuteMsg::StoreReceipt { receipt })?,
            funds: vec![],
        }))
    }
}

impl Module for ReceiptModule {
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
                reply_on: ReplyOn::Success,
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
            code_id: code_id.clone(),
            address: address.clone(),
        })),
        _ => Ok(None),
    }
}
