use cosmwasm_std::{to_binary, Api, Binary, Event, QuerierWrapper, StdError, Storage, SubMsg};
use serde::de::DeserializeOwned;

use crate::modules::ADOContract;
use common::{
    ado_base::{
        hooks::{AndromedaHook, HookMsg, OnFundsTransferResponse},
        modules::{Module, RECEIPT},
    },
    error::ContractError,
    Funds,
};

impl<'a> ADOContract<'a> {
    /// Sends the provided hook message to all registered modules
    pub fn module_hook<T: DeserializeOwned>(
        &self,
        storage: &dyn Storage,
        api: &dyn Api,
        querier: QuerierWrapper,
        hook_msg: AndromedaHook,
    ) -> Result<Vec<T>, ContractError> {
        let addresses: Vec<String> = self.load_module_addresses(storage, api, &querier)?;
        let mut resp: Vec<T> = Vec::new();
        for addr in addresses {
            let mod_resp = hook_query::<T>(&querier, hook_msg.clone(), addr)?;

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
        api: &dyn Api,
        querier: &QuerierWrapper,
        sender: String,
        amount: Funds,
        msg: Binary,
    ) -> Result<(Vec<SubMsg>, Vec<Event>, Funds), ContractError> {
        let modules: Vec<Module> = self.load_modules(storage)?;
        let mut remainder = amount;
        let mut msgs: Vec<SubMsg> = Vec::new();
        let mut events: Vec<Event> = Vec::new();
        let mut receipt_module_address: Option<String> = None;
        for module in modules {
            let app_contract = self.get_app_contract(storage)?;
            let module_address = module.address.get_address(api, querier, app_contract)?;
            if module.module_type == RECEIPT {
                // If receipt module exists we want to make sure we do it last.
                receipt_module_address = Some(module_address);
                continue;
            }
            let mod_resp: Option<OnFundsTransferResponse> = hook_query(
                querier,
                AndromedaHook::OnFundsTransfer {
                    payload: msg.clone(),
                    sender: sender.clone(),
                    amount: remainder.clone(),
                },
                module_address,
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
fn process_module_response<T>(
    mod_resp: Result<Option<T>, StdError>,
) -> Result<Option<T>, ContractError> {
    match mod_resp {
        Ok(mod_resp) => Ok(mod_resp),
        Err(StdError::NotFound { kind }) => {
            if kind.contains("operation") {
                Ok(None)
            } else {
                Err(ContractError::Std(StdError::NotFound { kind }))
            }
        }
        Err(e) => Err(e.into()),
    }
}

/// Queries the given address with the given hook message and returns the processed result.
fn hook_query<T: DeserializeOwned>(
    querier: &QuerierWrapper,
    hook_msg: AndromedaHook,
    addr: String,
) -> Result<Option<T>, ContractError> {
    let msg = HookMsg::AndrHook(hook_msg);
    let mod_resp: Result<Option<T>, StdError> = querier.query_wasm_smart(addr, &msg);
    process_module_response(mod_resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::error::ContractError;
    use cosmwasm_std::Response;

    #[test]
    fn test_process_module_response() {
        let res: Option<Response> = process_module_response(Ok(Some(Response::new()))).unwrap();
        assert_eq!(Some(Response::new()), res);

        let res: Option<Response> =
            process_module_response(Err(StdError::not_found("operation".to_string()))).unwrap();
        assert_eq!(None, res);

        let res: ContractError =
            process_module_response::<Response>(Err(StdError::generic_err("AnotherError")))
                .unwrap_err();
        assert_eq!(
            ContractError::Std(StdError::generic_err("AnotherError")),
            res
        );
    }
}
