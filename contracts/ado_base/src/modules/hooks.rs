use cosmwasm_std::{
    to_binary, Binary, Event, QuerierWrapper, QueryRequest, StdError, Storage, SubMsg, WasmQuery,
};
use serde::de::DeserializeOwned;

use crate::modules::ADOContract;
use andromeda_protocol::{
    ado_base::{
        hooks::{AndromedaHook, OnFundsTransferResponse},
        modules::{ModuleInfoWithAddress, ModuleType},
    },
    communication::HookMsg,
    error::ContractError,
    rates::Funds,
};

impl<'a> ADOContract<'a> {
    /// Sends the provided hook message to all registered modules
    pub fn module_hook<T>(
        &self,
        storage: &dyn Storage,
        querier: QuerierWrapper,
        hook_msg: AndromedaHook,
    ) -> Result<Vec<T>, ContractError>
    where
        T: DeserializeOwned,
    {
        let addresses: Vec<String> = self.load_module_addresses(storage)?;
        let mut resp: Vec<T> = Vec::new();
        for addr in addresses {
            let mod_resp: Option<T> = hook_query(querier, hook_msg.clone(), addr)?;
            if let Some(mod_resp) = mod_resp {
                resp.push(mod_resp);
            }
        }

        Ok(resp)
    }

    /// Sends the provided hook message to all registered modules
    pub fn on_funds_transfer(
        &self,
        storage: &dyn Storage,
        querier: QuerierWrapper,
        sender: String,
        amount: Funds,
        msg: Binary,
    ) -> Result<(Vec<SubMsg>, Vec<Event>, Funds), ContractError> {
        let modules: Vec<ModuleInfoWithAddress> = self.load_modules_with_address(storage)?;
        let mut remainder = amount;
        let mut msgs: Vec<SubMsg> = Vec::new();
        let mut events: Vec<Event> = Vec::new();
        let mut receipt_module_address: Option<String> = None;
        for module in modules {
            if module.module.module_type == ModuleType::Receipt {
                // If receipt module exists we want to make sure we do it last.
                receipt_module_address = Some(module.address.clone());
                continue;
            }
            let mod_resp: Option<OnFundsTransferResponse> = hook_query(
                querier,
                AndromedaHook::OnFundsTransfer {
                    payload: msg.clone(),
                    sender: sender.clone(),
                    amount: remainder.clone(),
                },
                module.address.clone(),
            )?;
            if let Some(mod_resp) = mod_resp {
                remainder = mod_resp.leftover_funds;
                msgs = [msgs, mod_resp.msgs].concat();
                events = [events, mod_resp.events].concat();
            }
        }
        if let Some(receipt_module_address) = receipt_module_address {
            let mod_resp: Option<OnFundsTransferResponse> = hook_query(
                querier,
                AndromedaHook::OnFundsTransfer {
                    payload: to_binary(&events)?,
                    sender,
                    amount: remainder.clone(),
                },
                receipt_module_address,
            )?;
            if let Some(mod_resp) = mod_resp {
                msgs = [msgs, mod_resp.msgs].concat();
                events = [events, mod_resp.events].concat();
            }
        }

        Ok((msgs, events, remainder))
    }
}

/// Processes the given module response by hiding the error if it is `UnsupportedOperation` and
/// bubbling up any other one. A return value of Ok(None) signifies that the operation was not
/// supported.
fn process_module_response<T>(mod_resp: Result<T, StdError>) -> Result<Option<T>, ContractError> {
    match mod_resp {
        Ok(mod_resp) => Ok(Some(mod_resp)),
        Err(StdError::GenericErr { msg }) => {
            if msg.contains("UnsupportedOperation") {
                Ok(None)
            } else {
                Err(ContractError::Std(StdError::GenericErr { msg }))
            }
        }
        Err(e) => Err(e.into()),
    }
}

/// Queriers the given address with the given hook message and returns the processed result.
fn hook_query<T>(
    querier: QuerierWrapper,
    hook_msg: AndromedaHook,
    addr: String,
) -> Result<Option<T>, ContractError>
where
    T: DeserializeOwned,
{
    let msg = HookMsg::AndrHook(hook_msg);
    let mod_resp: Result<T, StdError> = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: addr,
        msg: to_binary(&msg)?,
    }));
    process_module_response(mod_resp)
}
