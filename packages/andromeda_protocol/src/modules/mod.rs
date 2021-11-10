pub mod address_list;
pub mod common;
pub mod hooks;
pub mod receipt;
pub mod royalties;
pub mod taxable;

use crate::modules::{
    address_list::AddressListModule,
    hooks::{HookResponse, MessageHooks},
    receipt::ReceiptModule,
    royalties::Royalty,
    taxable::Taxable,
};
use cosmwasm_std::{StdResult, Storage, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MODULES: Item<Modules> = Item::new("modules");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
#[serde(rename_all = "snake_case")]
pub struct FlatRate {
    pub amount: Uint128,
    pub denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Rate {
    Flat(FlatRate),
    Percent(u64),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModuleDefinition {
    Whitelist {
        address: Option<String>,
        code_id: Option<u64>,
        moderators: Option<Vec<String>>,
    },
    Blacklist {
        address: Option<String>,
        code_id: Option<u64>,
        moderators: Option<Vec<String>>,
    },
    Taxable {
        rate: Rate,
        receivers: Vec<String>,
        description: Option<String>,
    },
    Royalties {
        rate: Rate,
        receivers: Vec<String>,
        description: Option<String>,
    },
    Receipt {
        address: Option<String>,
        code_id: Option<u64>,
        moderators: Option<Vec<String>>,
    },
}

pub trait Module: MessageHooks {
    fn validate(&self, modules: Vec<ModuleDefinition>) -> StdResult<bool>;
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
                moderators,
            } => Box::from(AddressListModule {
                moderators: moderators.clone(),
                address: address.clone(),
                code_id: code_id.clone(),
                inclusive: true,
            }),
            ModuleDefinition::Blacklist {
                address,
                code_id,
                moderators,
            } => Box::from(AddressListModule {
                moderators: moderators.clone(),
                address: address.clone(),
                code_id: code_id.clone(),
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
                moderators,
                address,
                code_id,
            } => Box::from(ReceiptModule {
                moderators: moderators.clone(),
                address: address.clone(),
                code_id: code_id.clone(),
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
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
    pub fn validate(&self) -> StdResult<bool> {
        for module in self.to_modules() {
            module.validate(self.module_defs.clone())?;
        }

        Ok(true)
    }
    pub fn hook<'a, F>(&self, f: F) -> StdResult<HookResponse>
    where
        F: Fn(Box<dyn Module>) -> StdResult<HookResponse>,
    {
        let modules = self.to_modules();
        let mut res = HookResponse::default();
        for module in modules {
            res = res.add_resp(f(module)?);
        }

        Ok(res)
    }
}

pub fn store_modules(storage: &mut dyn Storage, modules: Modules) -> StdResult<()> {
    //Validate each module before storing
    modules.validate()?;

    MODULES.save(storage, &modules)
}

pub fn read_modules(storage: &dyn Storage) -> StdResult<Modules> {
    let module_defs = MODULES.may_load(storage).unwrap_or_default();

    match module_defs {
        Some(mods) => Ok(mods),
        None => Ok(Modules::default()),
    }
}
