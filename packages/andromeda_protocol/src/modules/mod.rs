pub mod address_list;
pub mod common;
pub mod hooks;
pub mod receipt;

use ::common::error::ContractError;

use crate::modules::{
    address_list::AddressListModule,
    hooks::{HookResponse, MessageHooks},
    receipt::ReceiptModule,
};
use cosmwasm_std::{DepsMut, Env, MessageInfo, QuerierWrapper, StdResult, Storage};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MODULES: Item<Modules> = Item::new("modules");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// Definitions for each module, used in the `InstantiateMsg` for the token contract to define any modules assigned to the contract
pub enum ModuleDefinition {
    /// A whitelist module
    Whitelist {
        /// The address of the module contract
        address: Option<String>,
        /// A valid code ID for the module contract. Used upon contract instantiation to instantiate a new module contract.
        code_id: Option<u64>,
        /// A vector of contract operators. Used in combination with a valid `code_id` parameter
        operators: Option<Vec<String>>,
    },
    /// A blacklist module
    Blacklist {
        /// The address of the module contract
        address: Option<String>,
        /// A valid code ID for the module contract. Used upon contract instantiation to instantiate a new module contract.
        code_id: Option<u64>,
        /// A vector of contract operators. Used in combination with a valid `code_id` parameter
        operators: Option<Vec<String>>,
    },
    /// A receipt module
    Receipt {
        /// The address of the module contract
        address: Option<String>,
        /// A valid code ID for the module contract. Used upon contract instantiation to instantiate a new module contract.
        code_id: Option<u64>,
        /// A vector of contract operators. Used in combination with a valid `code_id` parameter
        operators: Option<Vec<String>>,
    },
}

pub trait Module: MessageHooks {
    fn validate(
        &self,
        modules: Vec<ModuleDefinition>,
        querier: &QuerierWrapper,
    ) -> Result<bool, ContractError>;
    fn as_definition(&self) -> ModuleDefinition;
    fn get_contract_address(&self, _storage: &dyn Storage) -> Option<String> {
        None
    }
}

impl ModuleDefinition {
    pub fn name(&self) -> String {
        String::from(match self {
            ModuleDefinition::Receipt { .. } => "receipt",
            ModuleDefinition::Whitelist { .. } => "whitelist",
            ModuleDefinition::Blacklist { .. } => "blacklist",
        })
    }
    pub fn as_module(&self) -> Box<dyn Module> {
        match self {
            ModuleDefinition::Whitelist {
                address,
                code_id,
                operators,
            } => Box::from(AddressListModule {
                operators: operators.clone(),
                address: address.clone(),
                // [MOD-01] Dereferencing the borrows and removing clone for u64.
                code_id: *code_id,
                inclusive: true,
            }),
            ModuleDefinition::Blacklist {
                address,
                code_id,
                operators,
            } => Box::from(AddressListModule {
                operators: operators.clone(),
                address: address.clone(),
                code_id: *code_id,
                inclusive: false,
            }),
            ModuleDefinition::Receipt {
                operators,
                address,
                code_id,
            } => Box::from(ReceiptModule {
                operators: operators.clone(),
                address: address.clone(),
                code_id: *code_id,
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// Helping struct to aid in hook execution.
/// The `Modules` struct implements all hooks that a `Module` may implement.
pub struct Modules {
    pub module_defs: Vec<ModuleDefinition>,
}

impl Modules {
    pub fn new(module_defs: Vec<ModuleDefinition>) -> Modules {
        Modules { module_defs }
    }
    pub fn default() -> Modules {
        Modules {
            module_defs: vec![],
        }
    }
    pub fn to_modules(&self) -> Vec<Box<dyn Module>> {
        self.module_defs
            .iter()
            .cloned()
            .map(|d| d.as_module())
            .collect()
    }
    pub fn validate(&self, querier: &QuerierWrapper) -> Result<bool, ContractError> {
        for module in self.to_modules() {
            module.validate(self.module_defs.clone(), querier)?;
        }

        Ok(true)
    }
    pub fn hook<F>(&self, f: F) -> Result<HookResponse, ContractError>
    where
        F: Fn(Box<dyn Module>) -> Result<HookResponse, ContractError>,
    {
        let modules = self.to_modules();
        let mut res = HookResponse::default();
        for module in modules {
            res = res.add_resp(f(module)?);
        }

        Ok(res)
    }
}

pub fn store_modules(
    storage: &mut dyn Storage,
    modules: Modules,
    querier: &QuerierWrapper,
) -> Result<(), ContractError> {
    //Validate each module before storing
    modules.validate(querier)?;

    Ok(MODULES.save(storage, &modules)?)
}

pub fn read_modules(storage: &dyn Storage) -> StdResult<Modules> {
    let module_defs = MODULES.may_load(storage).unwrap_or_default();

    match module_defs {
        Some(mods) => Ok(mods),
        None => Ok(Modules::default()),
    }
}

/// Generates instantiation messgaes for a list of modules
///
/// Returns a HookResponse object containing the instantiation messages
pub fn generate_instantiate_msgs(
    deps: &DepsMut,
    info: MessageInfo,
    env: Env,
    modules: Vec<Option<impl Module>>,
) -> Result<HookResponse, ContractError> {
    let mut resp = HookResponse::default();

    for module in modules.into_iter().flatten() {
        //On instantiate generates instantiation message for a module (if it is required)
        let hook_resp = module.on_instantiate(deps, info.clone(), env.clone())?;
        resp = resp.add_resp(hook_resp);
    }

    Ok(resp)
}
