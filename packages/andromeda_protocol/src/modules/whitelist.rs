use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError, StdResult, Storage};
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    modules::{
        common::{is_unique, require},
        hooks::{HookResponse, MessageHooks},
        {Module, ModuleDefinition},
    },
    token::ExecuteMsg,
};

use super::read_modules;

pub const WHITELIST: Map<String, bool> = Map::new("whitelist");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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

impl Module for Whitelist {
    fn validate(&self, all_modules: Vec<ModuleDefinition>) -> StdResult<bool> {
        require(
            is_unique(self, &all_modules),
            StdError::generic_err("Whitelist module must be unique"),
        )?;

        let contains_blacklist = all_modules.iter().any(|m| match m {
            &ModuleDefinition::Blacklist { .. } => true,
            _ => false,
        });
        require(
            !contains_blacklist,
            StdError::generic_err("Cannot have both a blacklist and a whitelist"),
        )?;

        Ok(true)
    }
    fn as_definition(&self) -> ModuleDefinition {
        ModuleDefinition::Whitelist {
            moderators: self.moderators.to_vec(),
        }
    }
}

impl MessageHooks for Whitelist {
    fn on_execute(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        _env: Env,
        _msg: ExecuteMsg,
    ) -> StdResult<HookResponse> {
        require(
            self.is_whitelisted(deps.storage, &info.sender.to_string())?
                || self.is_moderator(&info.sender.to_string()),
            StdError::generic_err("Address is not whitelisted"),
        )?;

        Ok(HookResponse::default())
    }
}

pub fn execute_whitelist(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
    whitelisted: bool,
) -> StdResult<Response> {
    let whitelist_def = get_whitelist_module(deps.storage)?;

    match whitelist_def {
        ModuleDefinition::Whitelist { moderators } => {
            let whitelist = Whitelist {
                moderators: moderators.to_vec(),
            };

            require(
                whitelist.is_moderator(&info.sender.to_string()),
                StdError::generic_err("Must be a moderator to whitelist an address"),
            )?;

            match whitelisted {
                true => whitelist.whitelist_addr(deps.storage, &address.to_string())?,
                false => whitelist.remove_whitelist(deps.storage, &address.to_string())?,
            };

            Ok(Response::default())
        }
        _ => Err(StdError::generic_err("Whitelist is improperly defined")),
    }
}

pub fn get_whitelist_module(storage: &dyn Storage) -> StdResult<ModuleDefinition> {
    let modules = read_modules(storage)?;
    let whitelist_def = modules
        .module_defs
        .iter()
        .find(|m| match m {
            ModuleDefinition::Whitelist { .. } => true,
            _ => false,
        })
        .ok_or(StdError::generic_err(
            "Token does not implement the whitelist module",
        ))?;

    Ok(whitelist_def.clone())
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
                description: None,
            },
        ];

        assert_eq!(wl.validate(modules.to_vec()), Ok(true));

        modules.push(ModuleDefinition::Whitelist { moderators: vec![] });

        assert_eq!(
            wl.validate(modules.to_vec()),
            Err(StdError::generic_err("Whitelist module must be unique"))
        );

        let modules = vec![
            wl.as_definition().clone(),
            ModuleDefinition::Taxable {
                tax: 2,
                receivers: vec![],
                description: None,
            },
            ModuleDefinition::Blacklist { moderators: vec![] },
        ];

        assert_eq!(
            wl.validate(modules.to_vec()),
            Err(StdError::generic_err(
                "Cannot have both a blacklist and a whitelist"
            ))
        );
    }

    #[test]
    fn test_on_execute() {
        let sender = String::from("sender");
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("sender", &[]);
        let wl = Whitelist { moderators: vec![] };
        let msg = ExecuteMsg::Revoke {
            spender: String::default(),
            token_id: String::default(),
        };

        let resp = wl
            .on_execute(&deps.as_mut(), info.clone(), env.clone(), msg.clone())
            .unwrap_err();

        assert_eq!(resp, StdError::generic_err("Address is not whitelisted"));

        wl.whitelist_addr(&mut deps.storage, &sender.clone())
            .unwrap();

        let resp = wl
            .on_execute(&deps.as_mut(), info.clone(), env.clone(), msg.clone())
            .unwrap();

        assert_eq!(resp, HookResponse::default());
    }
}
