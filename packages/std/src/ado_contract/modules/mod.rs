use std::convert::TryInto;

use crate::{
    ado_base::hooks::{AndromedaHook, HookMsg, OnFundsTransferResponse},
    ado_contract::state::ADOContract,
    common::Funds,
};
use cosmwasm_std::{
    ensure, Binary, Deps, Event, Order, QuerierWrapper, Response, StdError, Storage, SubMsg, Uint64,
};
use cw_storage_plus::Bound;
use serde::de::DeserializeOwned;

use crate::os::kernel::QueryMsg as KernelQueryMsg;
use crate::{ado_base::modules::Module, error::ContractError};

pub mod execute;
pub mod query;

impl<'a> ADOContract<'a> {
    /// Sends the provided hook message to all registered modules
    pub fn module_hook<T: DeserializeOwned>(
        &self,
        deps: &Deps,
        hook_msg: AndromedaHook,
    ) -> Result<Vec<T>, ContractError> {
        let addresses: Vec<String> = self.load_module_addresses(deps)?;
        let mut resp: Vec<T> = Vec::new();
        for addr in addresses {
            let mod_resp = hook_query::<T>(&deps.querier, hook_msg.clone(), addr)?;

            if let Some(mod_resp) = mod_resp {
                resp.push(mod_resp);
            }
        }

        Ok(resp)
    }

    /// Validates the given address for a module.
    pub(crate) fn validate_module_address(
        &self,
        deps: &Deps,
        module: &Module,
    ) -> Result<(), ContractError> {
        // Validate module is an ADO
        let addr = module.address.get_raw_address(deps)?;
        let query = KernelQueryMsg::VerifyAddress {
            address: addr.to_string(),
        };
        let kernel_addr = self.get_kernel_address(deps.storage)?;
        let res: bool = deps.querier.query_wasm_smart(kernel_addr, &query)?;
        ensure!(
            res,
            ContractError::InvalidModule {
                msg: Some(format!(
                    "Module {} is not a valid ADO",
                    module.name.clone().unwrap_or(module.address.to_string())
                ))
            }
        );
        Ok(())
    }

    pub fn register_modules(
        &self,
        sender: &str,
        storage: &mut dyn Storage,
        modules: Option<Vec<Module>>,
    ) -> Result<Response, ContractError> {
        let mut resp = Response::new();
        let modules = modules.unwrap_or_default();

        self.validate_modules(&modules)?;
        for module in modules {
            let register_response = self.execute_register_module(storage, sender, module, false)?;
            resp = resp
                .add_attributes(register_response.attributes)
                .add_submessages(register_response.messages)
        }

        Ok(resp)
    }

    /// Registers a module
    /// If the module has provided an address as its form of instantiation this address is recorded
    /// Each module is assigned a u64 index so as it can be unregistered/altered
    /// The assigned u64 index is used as the message id for use in the `reply` entry point of the contract
    fn register_module(
        &self,
        storage: &mut dyn Storage,
        module: &Module,
    ) -> Result<u64, ContractError> {
        let idx = self.module_idx.may_load(storage)?.unwrap_or(1);
        let idx_str = idx.to_string();
        self.module_info.save(storage, &idx_str, module)?;
        self.module_idx.save(storage, &(idx + 1))?;

        Ok(idx)
    }

    /// Deregisters a module.
    fn deregister_module(
        &self,
        storage: &mut dyn Storage,
        idx: Uint64,
    ) -> Result<(), ContractError> {
        let idx_str = idx.to_string();
        self.check_module_mutability(storage, &idx_str)?;
        self.module_info.remove(storage, &idx_str);

        Ok(())
    }

    /// Alters a module
    /// If the module has provided an address as its form of instantiation this address is recorded
    /// Each module is assigned a u64 index so as it can be unregistered/altered
    /// The assigned u64 index is used as the message id for use in the `reply` entry point of the contract
    fn alter_module(
        &self,
        storage: &mut dyn Storage,
        idx: Uint64,
        module: &Module,
    ) -> Result<(), ContractError> {
        let idx_str = idx.to_string();
        self.check_module_mutability(storage, &idx_str)?;
        self.module_info.save(storage, &idx_str, module)?;
        Ok(())
    }

    fn check_module_mutability(
        &self,
        storage: &dyn Storage,
        idx_str: &str,
    ) -> Result<(), ContractError> {
        let existing_module = self.module_info.may_load(storage, idx_str)?;
        match existing_module {
            None => return Err(ContractError::ModuleDoesNotExist {}),
            Some(m) => {
                if !m.is_mutable {
                    return Err(ContractError::ModuleImmutable {});
                }
            }
        }
        Ok(())
    }

    /// Loads all registered modules in Vector form
    pub(crate) fn load_modules(&self, storage: &dyn Storage) -> Result<Vec<Module>, ContractError> {
        // if !self.module_idx.may_load(storage)?.is_some() {
        //     return Ok(Vec::new());
        // }
        let module_idx = self.module_idx.may_load(storage)?.unwrap_or(1);
        let min = Some(Bound::inclusive("1"));
        let modules: Vec<Module> = self
            .module_info
            .range(storage, min, None, Order::Ascending)
            .take(module_idx.try_into().unwrap())
            .flatten()
            .map(|(_vec, module)| module)
            .collect();

        Ok(modules)
    }

    /// Loads all registered module addresses in Vector form
    fn load_module_addresses(&self, deps: &Deps) -> Result<Vec<String>, ContractError> {
        let vfs_address = self.get_vfs_address(deps.storage, &deps.querier)?;
        let module_addresses: Vec<String> = self
            .load_modules(deps.storage)?
            .into_iter()
            .map(|m| {
                m.address
                    .get_raw_address_from_vfs(deps, vfs_address.clone())
                    .unwrap()
                    .to_string()
            })
            .collect();

        Ok(module_addresses)
    }

    /// Validates all modules.
    fn validate_modules(&self, modules: &[Module]) -> Result<(), ContractError> {
        ensure!(
            modules.len() <= 100,
            ContractError::InvalidModules {
                msg: "Cannot have more than 100 modules".to_string()
            }
        );
        for module in modules {
            module.validate(modules)?;
        }

        Ok(())
    }

    /// Sends a `OnFundsTransfer` hook message to all registered modules.
    ///
    /// Returns a vector of all required sub messages from each of the registered modules.
    pub fn on_funds_transfer(
        &self,
        deps: &Deps,
        sender: String,
        amount: Funds,
        msg: Binary,
    ) -> Result<(Vec<SubMsg>, Vec<Event>, Funds), ContractError> {
        let mut remainder = amount;
        let mut msgs: Vec<SubMsg> = Vec::new();
        let mut events: Vec<Event> = Vec::new();

        let vfs_address = self.get_vfs_address(deps.storage, &deps.querier)?;
        let modules: Vec<Module> = self.load_modules(deps.storage)?;
        for module in modules {
            let module_address = module
                .address
                .get_raw_address_from_vfs(deps, vfs_address.clone())?;
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
    use crate::testing::mock_querier::{mock_dependencies_custom, MOCK_APP_CONTRACT};
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_info},
        to_binary, Addr, Coin,
    };

    #[test]
    fn test_execute_register_module_unauthorized() {
        let mut deps = mock_dependencies();

        let module = Module::new("address_list", "address", false);
        let deps_mut = deps.as_mut();
        ADOContract::default()
            .owner
            .save(deps_mut.storage, &Addr::unchecked("owner"))
            .unwrap();
        ADOContract::default()
            .ado_type
            .save(deps_mut.storage, &"cw20".to_string())
            .unwrap();

        let res = ADOContract::default().execute_register_module(
            deps_mut.storage,
            "sender",
            module,
            true,
        );

        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_register_module_addr() {
        let mut deps = mock_dependencies();

        let module = Module::new("address_list", "address", false);
        let deps_mut = deps.as_mut();
        ADOContract::default()
            .owner
            .save(deps_mut.storage, &Addr::unchecked("owner"))
            .unwrap();

        ADOContract::default()
            .ado_type
            .save(deps_mut.storage, &"cw20".to_string())
            .unwrap();

        let res = ADOContract::default()
            .execute_register_module(deps_mut.storage, "owner", module.clone(), true)
            .unwrap();

        assert_eq!(
            Response::default()
                .add_attribute("action", "register_module")
                .add_attribute("module_idx", "1"),
            res
        );

        assert_eq!(
            module,
            ADOContract::default()
                .module_info
                .load(deps.as_mut().storage, "1")
                .unwrap()
        );
    }

    #[test]
    fn test_execute_alter_module_unauthorized() {
        let mut deps = mock_dependencies();
        let info = mock_info("sender", &[]);
        let module = Module::new("address_list", "address", true);
        ADOContract::default()
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        ADOContract::default()
            .ado_type
            .save(deps.as_mut().storage, &"cw20".to_string())
            .unwrap();

        let res =
            ADOContract::default().execute_alter_module(deps.as_mut(), info, 1u64.into(), module);

        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_alter_module_addr() {
        let mut deps = mock_dependencies();
        let info = mock_info("owner", &[]);
        let module = Module::new("address_list", "address", true);

        ADOContract::default()
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        ADOContract::default()
            .module_info
            .save(deps.as_mut().storage, "1", &module)
            .unwrap();
        ADOContract::default()
            .ado_type
            .save(deps.as_mut().storage, &"cw20".to_string())
            .unwrap();

        let module = Module::new("receipt", "other_address", true);

        let res = ADOContract::default()
            .execute_alter_module(deps.as_mut(), info, 1u64.into(), module.clone())
            .unwrap();

        assert_eq!(
            Response::default()
                .add_attribute("action", "alter_module")
                .add_attribute("module_idx", "1"),
            res
        );

        assert_eq!(
            module,
            ADOContract::default()
                .module_info
                .load(deps.as_mut().storage, "1")
                .unwrap()
        );
    }

    #[test]
    fn test_execute_alter_module_immutable() {
        let mut deps = mock_dependencies();
        let info = mock_info("owner", &[]);
        let module = Module::new("address_list", "address", false);

        ADOContract::default()
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        ADOContract::default()
            .module_info
            .save(deps.as_mut().storage, "1", &module)
            .unwrap();
        ADOContract::default()
            .ado_type
            .save(deps.as_mut().storage, &"cw20".to_string())
            .unwrap();

        let module = Module::new("receipt", "other_address", true);

        let res =
            ADOContract::default().execute_alter_module(deps.as_mut(), info, 1u64.into(), module);

        assert_eq!(ContractError::ModuleImmutable {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_alter_module_nonexisting_module() {
        let mut deps = mock_dependencies();
        let info = mock_info("owner", &[]);
        let module = Module::new("auction", "address", true);

        ADOContract::default()
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();
        ADOContract::default()
            .ado_type
            .save(deps.as_mut().storage, &"cw20".to_string())
            .unwrap();

        let res =
            ADOContract::default().execute_alter_module(deps.as_mut(), info, 1u64.into(), module);

        assert_eq!(ContractError::ModuleDoesNotExist {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_deregister_module_unauthorized() {
        let mut deps = mock_dependencies();
        let info = mock_info("sender", &[]);
        ADOContract::default()
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let res =
            ADOContract::default().execute_deregister_module(deps.as_mut(), info, 1u64.into());

        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_deregister_module() {
        let mut deps = mock_dependencies();
        let info = mock_info("owner", &[]);
        ADOContract::default()
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let module = Module::new("address_list", "address", true);

        ADOContract::default()
            .module_info
            .save(deps.as_mut().storage, "1", &module)
            .unwrap();

        let res = ADOContract::default()
            .execute_deregister_module(deps.as_mut(), info, 1u64.into())
            .unwrap();

        assert_eq!(
            Response::default()
                .add_attribute("action", "deregister_module")
                .add_attribute("module_idx", "1"),
            res
        );

        assert!(!ADOContract::default()
            .module_info
            .has(deps.as_mut().storage, "1"));
    }

    #[test]
    fn test_execute_deregister_module_immutable() {
        let mut deps = mock_dependencies();
        let info = mock_info("owner", &[]);
        ADOContract::default()
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let module = Module::new("address_list", "address", false);

        ADOContract::default()
            .module_info
            .save(deps.as_mut().storage, "1", &module)
            .unwrap();

        let res =
            ADOContract::default().execute_deregister_module(deps.as_mut(), info, 1u64.into());
        assert_eq!(ContractError::ModuleImmutable {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_deregister_module_nonexisting_module() {
        let mut deps = mock_dependencies();
        let info = mock_info("owner", &[]);
        ADOContract::default()
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let res =
            ADOContract::default().execute_deregister_module(deps.as_mut(), info, 1u64.into());

        assert_eq!(ContractError::ModuleDoesNotExist {}, res.unwrap_err());
    }

    #[test]
    fn test_load_module_addresses() {
        let mut deps = mock_dependencies_custom(&[]);
        let contract = ADOContract::default();

        let resp = contract.load_module_addresses(&deps.as_ref()).unwrap();
        assert!(resp.is_empty());
        contract
            .app_contract
            .save(deps.as_mut().storage, &Addr::unchecked(MOCK_APP_CONTRACT))
            .unwrap();
        contract.module_idx.save(deps.as_mut().storage, &2).unwrap();
        contract
            .module_info
            .save(
                deps.as_mut().storage,
                "1",
                &Module::new("address_list", "address", true),
            )
            .unwrap();

        contract
            .module_info
            .save(
                deps.as_mut().storage,
                "2",
                &Module::new("address_list", "address2", true),
            )
            .unwrap();
        let module_addresses = contract.load_module_addresses(&deps.as_ref()).unwrap();

        assert_eq!(
            vec![String::from("address"), String::from("address2")],
            module_addresses
        );
    }

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

    #[test]
    fn test_validate_modules() {
        let mut modules = vec![];
        let err = ADOContract::default()
            .validate_modules(&modules)
            .unwrap_err();
        assert_eq!(
            err,
            ContractError::InvalidModules {
                msg: "Must provide at least one module".to_string()
            }
        );

        let mut i = 0;
        while i < 101 {
            modules.push(Module::new(i.to_string(), i.to_string(), true));
            i += 1;
        }

        let err = ADOContract::default()
            .validate_modules(&modules)
            .unwrap_err();
        assert_eq!(
            err,
            ContractError::InvalidModules {
                msg: "Cannot have more than 100 modules".to_string()
            }
        );

        modules.clear();
        modules.push(Module::new("address_list", "address", true));
        modules.push(Module::new("receipt", "address", true));
        modules.push(Module::new("auction", "address", true));

        let res = ADOContract::default().validate_modules(&modules);
        assert!(res.is_ok());
    }

    #[test]
    fn test_module_hook() {
        let deps = mock_dependencies_custom(&[]);
        let contract = ADOContract::default();

        let resp: Vec<String> = contract
            .module_hook(
                &deps.as_ref(),
                AndromedaHook::OnFundsTransfer {
                    payload: to_binary(&true).unwrap(),
                    sender: "sender".to_string(),
                    amount: Funds::Native(Coin::new(100u128, "uandr")),
                },
            )
            .unwrap();

        assert!(resp.is_empty());
    }
}
