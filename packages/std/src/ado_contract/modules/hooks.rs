use cosmwasm_std::{Api, Binary, Deps, Event, QuerierWrapper, StdError, Storage, SubMsg};
use serde::de::DeserializeOwned;

use crate::ado_contract::modules::ADOContract;
use crate::{
    ado_base::{
        hooks::{AndromedaHook, HookMsg, OnFundsTransferResponse},
        modules::Module,
    },
    common::Funds,
    error::ContractError,
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
        deps: &Deps,
        sender: String,
        amount: Funds,
        msg: Binary,
    ) -> Result<(Vec<SubMsg>, Vec<Event>, Funds), ContractError> {
        let modules: Vec<Module> = self.load_modules(deps.storage)?;
        let mut remainder = amount;
        let mut msgs: Vec<SubMsg> = Vec::new();
        let mut events: Vec<Event> = Vec::new();
        for module in modules {
            let module_address = module.address.get_raw_address(deps)?;
            let mod_resp: Option<OnFundsTransferResponse> = hook_query(
                &deps.querier,
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
    addr: impl Into<String>,
) -> Result<Option<T>, ContractError> {
    let msg = HookMsg::AndrHook(hook_msg);
    let mod_resp: Result<Option<T>, StdError> = querier.query_wasm_smart(addr, &msg);
    process_module_response(mod_resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ContractError;
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
