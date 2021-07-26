use cosmwasm_std::{HumanAddr, StdError, StdResult, Storage};
use cosmwasm_storage::{bucket, bucket_read};

use crate::{
    common::{is_unique, require},
    modules::{Module, ModuleDefinition},
};

const WHITELIST_NS: &[u8] = b"whitelist";

pub struct Whitelist {
    pub moderators: Vec<HumanAddr>,
}

impl Whitelist {
    fn is_moderator(&self, addr: &HumanAddr) -> bool {
        self.moderators.contains(addr)
    }
    fn whitelist_addr<S: Storage>(&self, storage: &mut S, addr: &HumanAddr) -> StdResult<()> {
        bucket(WHITELIST_NS, storage).save(addr.to_string().as_bytes(), &true)
    }
    fn remove_whitelist<S: Storage>(&self, storage: &mut S, addr: &HumanAddr) -> StdResult<()> {
        bucket(WHITELIST_NS, storage).save(addr.to_string().as_bytes(), &false)
    }
    fn is_whitelisted<S: Storage>(&self, storage: &S, addr: &HumanAddr) -> StdResult<bool> {
        match bucket_read(WHITELIST_NS, storage).load(addr.to_string().as_bytes()) {
            Ok(whitelisted) => Ok(whitelisted),
            Err(e) => match e {
                cosmwasm_std::StdError::NotFound { .. } => Ok(false),
                _ => Err(e),
            },
        }
    }
}

impl Module for Whitelist {
    fn validate(&self, all_modules: Vec<ModuleDefinition>) -> StdResult<bool> {
        require(
            is_unique(self, &all_modules),
            StdError::generic_err("Whitelist module must be unique"),
        )?;

        Ok(true)
    }
    fn as_definition(&self) -> ModuleDefinition {
        ModuleDefinition::WhiteList {
            moderators: self.moderators.to_vec(),
        }
    }
}
