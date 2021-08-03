use cosmwasm_std::{Api, DepsMut, Env, MessageInfo, Querier, StdError, StdResult, Storage};
use cw_storage_plus::Map;

use crate::{
    common::{is_unique, require},
    hooks::{HookResponse, Payments, PreHooks},
    modules::{Module, ModuleDefinition},
};

const WHITELIST_NS: &[u8] = b"whitelist";
const WHITELIST: Map<String, bool> = Map::new("whitelist");

pub struct Whitelist {
    pub moderators: Vec<String>,
}

impl Whitelist {
    pub fn is_moderator(&self, addr: &String) -> bool {
        self.moderators.contains(addr)
    }
    pub fn whitelist_addr(&self, storage: &mut dyn Storage, addr: &String) -> StdResult<()> {
        WHITELIST.save(storage, addr.clone(), &true)
    }
    pub fn remove_whitelist(&self, storage: &mut dyn Storage, addr: &String) -> StdResult<()> {
        WHITELIST.save(storage, addr.clone(), &false)
    }
    pub fn is_whitelisted(&self, storage: &dyn Storage, addr: &String) -> StdResult<bool> {
        match WHITELIST.load(storage, addr.clone()) {
            Ok(whitelisted) => Ok(whitelisted),
            Err(e) => match e {
                cosmwasm_std::StdError::NotFound { .. } => Ok(false),
                _ => Err(e),
            },
        }
    }
}

impl PreHooks for Whitelist {
    fn pre_execute(&self, deps: &DepsMut, info: MessageInfo, _env: Env) -> StdResult<HookResponse> {
        require(
            self.is_whitelisted(deps.storage, &info.sender.to_string())?,
            StdError::generic_err("Address is not whitelisted"),
        )?;

        Ok(HookResponse::default())
    }
}

impl Payments for Whitelist {}

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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn test_validate() {
        let wl = Whitelist { moderators: vec![] };
        let mut modules = vec![
            wl.as_definition().clone(),
            ModuleDefinition::Taxable {
                tax: 2,
                receivers: vec![],
            },
        ];

        assert_eq!(wl.validate(modules.to_vec()), Ok(true));

        modules.push(ModuleDefinition::WhiteList { moderators: vec![] });

        assert_eq!(
            wl.validate(modules.to_vec()),
            Err(StdError::generic_err("Whitelist module must be unique"))
        );
    }

    #[test]
    fn test_pre_handle() {
        let sender = String::from("sender");
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("sender", &[]);
        let wl = Whitelist { moderators: vec![] };

        let resp = wl
            .pre_execute(&deps.as_mut(), info.clone(), env.clone())
            .unwrap_err();

        assert_eq!(resp, StdError::generic_err("Address is not whitelisted"));

        wl.whitelist_addr(&mut deps.storage, &sender.clone())
            .unwrap();

        let resp = wl
            .pre_execute(&deps.as_mut(), info.clone(), env.clone())
            .unwrap();

        assert_eq!(resp, HookResponse::default());
    }
}
