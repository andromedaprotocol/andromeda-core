use crate::{
    hooks::{Payments, PreHooks},
    whitelist::Whitelist,
};
use cosmwasm_std::{HumanAddr, StdError, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const KEY_MODULES: &[u8] = b"modules";

pub type Fee = u128;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
pub enum ModuleDefinition {
    WhiteList { moderators: Vec<HumanAddr> },
    Taxable { tax: Fee, receivers: Vec<HumanAddr> },
    // Royalties { fee: Fee, receivers: Vec<HumanAddr> },
}

//Converts a ModuleDefinition to a Module struct
pub fn as_module(definition: ModuleDefinition) -> impl Module {
    match definition {
        ModuleDefinition::WhiteList { moderators } => Whitelist { moderators },
        ModuleDefinition::Taxable { .. } => Whitelist { moderators: vec![] },
    }
}

//Converts a vector of ModuleDefinitions to a vector of Module structs
pub fn as_modules(definitions: Vec<ModuleDefinition>) -> Vec<impl Module> {
    definitions.into_iter().map(|d| as_module(d)).collect()
}
pub trait Module: PreHooks + Payments {
    fn validate(&self, extensions: Vec<ModuleDefinition>) -> StdResult<bool>;
    fn as_definition(&self) -> ModuleDefinition;
}

pub fn store_modules<S: Storage>(
    storage: &mut S,
    module_defs: Vec<ModuleDefinition>,
) -> StdResult<()> {
    //Validate each module before storing
    let modules = as_modules(module_defs.clone());
    for module in modules {
        module.validate(module_defs.clone())?;
    }

    singleton(storage, KEY_MODULES).save(&module_defs)
}

pub fn read_modules<S: Storage>(storage: &S) -> StdResult<Vec<impl Module>> {
    match singleton_read(storage, KEY_MODULES).load() {
        Ok(defs) => Ok(as_modules(defs)),
        Err(err) => match err {
            StdError::NotFound { .. } => Ok(vec![]),
            _ => Err(err),
        },
    }
}
