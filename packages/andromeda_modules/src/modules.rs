use crate::taxable::Taxable;
use crate::{
    hooks::{Payments, PreHooks},
    whitelist::Whitelist,
};
use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// const KEY_MODULES: &[u8] = b"modules";
pub const MODULES: Item<Vec<ModuleDefinition>> = Item::new("modules");

pub type Fee = u128;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
pub enum ModuleDefinition {
    WhiteList { moderators: Vec<String> },
    Taxable { tax: Fee, receivers: Vec<String> },
    // Royalties { fee: Fee, receivers: Vec<String> },
}

//Converts a ModuleDefinition to a Module struct
pub fn as_module(definition: ModuleDefinition) -> Box<dyn Module> {
    match definition {
        ModuleDefinition::WhiteList { moderators } => Box::from(Whitelist { moderators }),
        ModuleDefinition::Taxable { tax, receivers } => Box::from(Taxable { tax, receivers }),
    }
}

//Converts a vector of ModuleDefinitions to a vector of Module structs
pub fn as_modules(definitions: Vec<ModuleDefinition>) -> Vec<Box<dyn Module>> {
    definitions.into_iter().map(|d| as_module(d)).collect()
}
pub trait Module: PreHooks + Payments {
    fn validate(&self, extensions: Vec<ModuleDefinition>) -> StdResult<bool>;
    fn as_definition(&self) -> ModuleDefinition;
}

pub fn store_modules(
    storage: &mut dyn Storage,
    module_defs: &Vec<ModuleDefinition>,
) -> StdResult<()> {
    //Validate each module before storing
    let modules = as_modules(module_defs.clone());
    for module in modules {
        module.validate(module_defs.clone())?;
    }

    MODULES.save(storage, module_defs)
}

pub fn read_modules(storage: &dyn Storage) -> StdResult<Vec<Box<dyn Module>>> {
    let module_defs = MODULES.may_load(storage).unwrap_or_default();

    match module_defs {
        Some(defs) => Ok(as_modules(defs)),
        None => Ok(vec![]),
    }
}
