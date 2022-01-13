pub mod address_list;
pub mod common;
pub mod hooks;
pub mod receipt;
pub mod royalties;
pub mod taxable;

use crate::{
    error::ContractError,
    modules::{
        address_list::AddressListModule,
        hooks::{HookResponse, MessageHooks},
        receipt::ReceiptModule,
        royalties::Royalty,
        taxable::Taxable,
    },
    require,
};
use cosmwasm_std::{DepsMut, Env, MessageInfo, StdResult, Storage, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MODULES: Item<Modules> = Item::new("modules");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
#[serde(rename_all = "snake_case")]
/// A struct used to define a flat rate fee
pub struct FlatRate {
    /// The fee amount
    pub amount: Uint128,
    /// The fee denomination
    pub denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
#[serde(rename_all = "snake_case")]
/// An enum used to define various types of fees
pub enum Rate {
    /// A flat rate fee
    Flat(FlatRate),
    /// A percentage fee (integer)
    Percent(u64),
}

impl Rate {
    /// Validates that a given rate is non-zero
    pub fn is_non_zero(&self) -> bool {
        match self {
            Rate::Flat(rate) => rate.amount.u128() > 0,
            Rate::Percent(rate) => rate > &0,
        }
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        require(self.is_non_zero(), ContractError::InvalidRate {})?;

        if let Rate::Percent(rate) = self {
            require(rate <= &100, ContractError::InvalidRate {})?;
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
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
    /// A tax module. Required payments are paid by the purchaser.
    Taxable {
        /// The tax rate
        rate: Rate,
        /// The receiving addresses of the fee
        receivers: Vec<String>,
        /// An optional description of the fee
        description: Option<String>,
    },
    /// A royalty module. Required payments are paid by the seller.
    Royalties {
        /// The royalty rate
        rate: Rate,
        /// The receiving addresses of the fee
        receivers: Vec<String>,
        /// An optional description of the fee
        description: Option<String>,
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
    fn validate(&self, modules: Vec<ModuleDefinition>) -> Result<bool, ContractError>;
    fn as_definition(&self) -> ModuleDefinition;
    fn get_contract_address(&self, _storage: &dyn Storage) -> Option<String> {
        None
    }
}

impl ModuleDefinition {
    pub fn name(&self) -> String {
        String::from(match self {
            ModuleDefinition::Receipt { .. } => "receipt",
            ModuleDefinition::Royalties { .. } => "royalty",
            ModuleDefinition::Whitelist { .. } => "whitelist",
            ModuleDefinition::Blacklist { .. } => "blacklist",
            ModuleDefinition::Taxable { .. } => "tax",
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
            ModuleDefinition::Taxable {
                rate,
                receivers,
                description,
            } => Box::from(Taxable {
                rate: rate.clone(),
                receivers: receivers.clone(),
                description: description.clone(),
            }),
            ModuleDefinition::Royalties {
                rate,
                receivers,
                description,
            } => Box::from(Royalty {
                rate: rate.clone(),
                receivers: receivers.to_vec(),
                description: description.clone(),
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
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
            .to_vec()
            .into_iter()
            .map(|d| d.as_module())
            .collect()
    }
    pub fn validate(&self) -> Result<bool, ContractError> {
        for module in self.to_modules() {
            module.validate(self.module_defs.clone())?;
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

pub fn store_modules(storage: &mut dyn Storage, modules: Modules) -> Result<(), ContractError> {
    //Validate each module before storing
    modules.validate()?;

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
