use std::convert::TryInto;

use crate::state::ADOContract;
use cosmwasm_std::{Api, DepsMut, MessageInfo, Order, QuerierWrapper, Response, Storage, Uint64};
use cw_storage_plus::Bound;

use common::{ado_base::modules::Module, error::ContractError, require};

pub mod hooks;

impl<'a> ADOContract<'a> {
    pub fn register_modules(
        &self,
        sender: &str,
        storage: &mut dyn Storage,
        modules: Vec<Module>,
    ) -> Result<Response, ContractError> {
        self.validate_modules(&modules, &self.ado_type.load(storage)?)?;
        let mut resp = Response::new();
        for module in modules {
            let register_response = self.execute_register_module(storage, sender, module, false)?;
            resp = resp
                .add_attributes(register_response.attributes)
                .add_submessages(register_response.messages)
        }
        Ok(resp)
    }

    /// A wrapper for `fn register_module`. The parameters are "extracted" from `DepsMut` to be able to
    /// execute this in a loop without cloning.
    pub(crate) fn execute_register_module(
        &self,
        storage: &mut dyn Storage,
        sender: &str,
        module: Module,
        should_validate: bool,
    ) -> Result<Response, ContractError> {
        require(
            self.is_owner_or_operator(storage, sender)?,
            ContractError::Unauthorized {},
        )?;
        let resp = Response::default();
        let idx = self.register_module(storage, &module)?;
        if should_validate {
            self.validate_modules(&self.load_modules(storage)?, &self.ado_type.load(storage)?)?;
        }
        Ok(resp
            .add_attribute("action", "register_module")
            .add_attribute("module_idx", idx.to_string()))
    }

    /// A wrapper for `fn alter_module`.
    pub(crate) fn execute_alter_module(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        module_idx: Uint64,
        module: Module,
    ) -> Result<Response, ContractError> {
        let addr = info.sender.as_str();
        require(
            self.is_owner_or_operator(deps.storage, addr)?,
            ContractError::Unauthorized {},
        )?;
        self.alter_module(deps.storage, module_idx, &module)?;
        self.validate_modules(
            &self.load_modules(deps.storage)?,
            &self.ado_type.load(deps.storage)?,
        )?;
        Ok(Response::default()
            .add_attribute("action", "alter_module")
            .add_attribute("module_idx", module_idx))
    }

    /// A wrapper for `fn deregister_module`.
    pub(crate) fn execute_deregister_module(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        module_idx: Uint64,
    ) -> Result<Response, ContractError> {
        let addr = info.sender.as_str();
        require(
            self.is_owner_or_operator(deps.storage, addr)?,
            ContractError::Unauthorized {},
        )?;
        self.deregister_module(deps.storage, module_idx)?;
        Ok(Response::default()
            .add_attribute("action", "deregister_module")
            .add_attribute("module_idx", module_idx))
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
    fn load_modules(&self, storage: &dyn Storage) -> Result<Vec<Module>, ContractError> {
        let module_idx = self.module_idx.may_load(storage)?.unwrap_or(1);
        let min = Some(Bound::Inclusive(1u64.to_le_bytes().to_vec()));
        // let max = Some(Bound::Inclusive(1u64.to_le_bytes().to_vec()));
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
    fn load_module_addresses(
        &self,
        storage: &dyn Storage,
        api: &dyn Api,
        querier: &QuerierWrapper,
    ) -> Result<Vec<String>, ContractError> {
        let mission_contract = self.get_mission_contract(storage)?;
        let module_addresses: Result<Vec<String>, _> = self
            .load_modules(storage)?
            .iter()
            .map(|m| {
                m.address
                    .get_address(api, querier, mission_contract.clone())
            })
            .collect();

        module_addresses
    }

    /*
     * TODO: Remove when happy with InstantiateType removal.
     * /// Loads all modules with their registered addresses in Vector form
    fn load_modules_with_address(
        &self,
        storage: &dyn Storage,
    ) -> Result<Vec<ModuleInfoWithAddress>, ContractError> {
        let modules = self.load_modules(storage)?;
        let module_idx = self.module_idx.may_load(storage)?.unwrap_or(1);
        let min = Some(Bound::Inclusive(1u64.to_le_bytes().to_vec()));
        // let max = Some(Bound::Inclusive(1u64.to_le_bytes().to_vec()));
        let module_addresses: Vec<String> = self
            .module_addr
            .range(storage, min, None, Order::Ascending)
            .take(module_idx.try_into().unwrap())
            .flatten()
            .map(|(_vec, addr)| addr.to_string())
            .collect();

        let mut modules_with_addresses: Vec<ModuleInfoWithAddress> = Vec::new();
        for (index, module_address) in module_addresses.iter().enumerate() {
            let module_opt = modules.get(index);
            if let Some(module) = module_opt {
                modules_with_addresses.push(ModuleInfoWithAddress {
                    module: module.clone(),
                    address: module_address.to_string(),
                });
            }
        }

        Ok(modules_with_addresses)
    }*/

    /// Validates all modules.
    pub fn validate_modules(
        &self,
        modules: &[Module],
        ado_type: &str,
    ) -> Result<(), ContractError> {
        for module in modules {
            module.validate(modules, ado_type)?;
        }

        Ok(())
    }

    /*
     * TODO: Remove when happyw with InstantiateType removal
     * pub fn handle_module_reply(
        &self,
        deps: DepsMut,
        msg: Reply,
    ) -> Result<Response, ContractError> {
        if msg.result.is_err() {
            return Err(ContractError::Std(StdError::generic_err(
                msg.result.unwrap_err(),
            )));
        }

        let id = msg.id.to_string();
        require(
            self.module_info.has(deps.storage, &id),
            ContractError::InvalidReplyId {},
        )?;

        let addr = get_reply_address(&msg)?;
        self.module_addr
            .save(deps.storage, &id, &deps.api.addr_validate(&addr)?)?;

        Ok(Response::default())
    }*/
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{
        ado_base::modules::{ADDRESS_LIST, AUCTION, RECEIPT},
        mission::AndrAddress,
    };
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_info},
        Addr,
    };

    #[test]
    fn test_execute_register_module_unauthorized() {
        let mut deps = mock_dependencies(&[]);

        let module = Module {
            module_type: ADDRESS_LIST.to_owned(),
            address: AndrAddress {
                identifier: "address".to_string(),
            },
            is_mutable: false,
        };
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
        let mut deps = mock_dependencies(&[]);

        let module = Module {
            module_type: ADDRESS_LIST.to_owned(),
            address: AndrAddress {
                identifier: "address".to_string(),
            },
            is_mutable: false,
        };
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
    fn test_execute_register_module_validate() {
        let mut deps = mock_dependencies(&[]);

        let module = Module {
            module_type: AUCTION.to_owned(),
            address: AndrAddress {
                identifier: "address".to_string(),
            },
            is_mutable: false,
        };
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
            "owner",
            module.clone(),
            true,
        );

        assert_eq!(
            ContractError::IncompatibleModules {
                msg: "An Auction module cannot be used for a CW20 ADO".to_string()
            },
            res.unwrap_err(),
        );

        let _res = ADOContract::default()
            .execute_register_module(deps_mut.storage, "owner", module, false)
            .unwrap();
    }

    #[test]
    fn test_execute_alter_module_unauthorized() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("sender", &[]);
        let module = Module {
            module_type: ADDRESS_LIST.to_owned(),
            address: AndrAddress {
                identifier: "address".to_string(),
            },
            is_mutable: true,
        };
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
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        let module = Module {
            module_type: ADDRESS_LIST.to_owned(),
            address: AndrAddress {
                identifier: "address".to_string(),
            },
            is_mutable: true,
        };

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

        let module = Module {
            module_type: RECEIPT.to_owned(),
            address: AndrAddress {
                identifier: "other_address".to_string(),
            },
            is_mutable: true,
        };

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
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        let module = Module {
            module_type: ADDRESS_LIST.to_owned(),
            address: AndrAddress {
                identifier: "address".to_string(),
            },
            is_mutable: false,
        };

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

        let module = Module {
            module_type: RECEIPT.to_owned(),
            address: AndrAddress {
                identifier: "other_address".to_string(),
            },
            is_mutable: true,
        };

        let res =
            ADOContract::default().execute_alter_module(deps.as_mut(), info, 1u64.into(), module);

        assert_eq!(ContractError::ModuleImmutable {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_alter_module_nonexisting_module() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        let module = Module {
            module_type: AUCTION.to_owned(),
            address: AndrAddress {
                identifier: "address".to_string(),
            },
            is_mutable: true,
        };

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
    fn test_execute_alter_module_incompatible_module() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        let module = Module {
            module_type: AUCTION.to_owned(),
            address: AndrAddress {
                identifier: "address".to_string(),
            },
            is_mutable: true,
        };

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

        let res =
            ADOContract::default().execute_alter_module(deps.as_mut(), info, 1u64.into(), module);

        assert_eq!(
            ContractError::IncompatibleModules {
                msg: "An Auction module cannot be used for a CW20 ADO".to_string()
            },
            res.unwrap_err(),
        );
    }

    #[test]
    fn test_execute_deregister_module_unauthorized() {
        let mut deps = mock_dependencies(&[]);
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
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        ADOContract::default()
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let module = Module {
            module_type: ADDRESS_LIST.to_owned(),
            address: AndrAddress {
                identifier: "address".to_string(),
            },
            is_mutable: true,
        };

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
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        ADOContract::default()
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let module = Module {
            module_type: ADDRESS_LIST.to_owned(),
            address: AndrAddress {
                identifier: "address".to_string(),
            },
            is_mutable: false,
        };

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
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        ADOContract::default()
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let res =
            ADOContract::default().execute_deregister_module(deps.as_mut(), info, 1u64.into());

        assert_eq!(ContractError::ModuleDoesNotExist {}, res.unwrap_err());
    }
}
