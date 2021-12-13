use cosmwasm_std::{StdError, StdResult};

use super::{
    common::{is_unique, require},
    hooks::MessageHooks,
    Module, ModuleDefinition,
};

pub struct MetadataStorage {
    pub size_limit: Option<u64>,
    pub description: Option<String>,
}

impl MessageHooks for MetadataStorage {}

impl Module for MetadataStorage {
    fn validate(&self, modules: Vec<ModuleDefinition>) -> StdResult<bool> {
        require(
            is_unique(self, &modules),
            StdError::generic_err("Metadata Storage module must be unique"),
        )?;

        Ok(true)
    }
    fn as_definition(&self) -> ModuleDefinition {
        ModuleDefinition::MetadataStorage {
            size_limit: self.size_limit,
            description: self.description,
        }
    }
}
