use cosmwasm_std::{
    to_binary, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, StdError, StdResult, Storage,
    SubMsg, WasmMsg,
};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::read_modules;
use crate::response::get_reply_address;
use crate::{
    address_list::{query_includes_address, InstantiateMsg as AddressListInstantiateMsg},
    modules::{
        common::is_unique,
        hooks::{HookResponse, MessageHooks},
        {Module, ModuleDefinition},
    },
    require::require,
};

pub const ADDRESS_LIST_CONTRACT: Item<String> = Item::new("addresslistcontract");
pub const REPLY_ADDRESS_LIST: u64 = 2;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AddressListModule {
    pub address: Option<String>,
    pub code_id: Option<u64>,
    pub moderators: Option<Vec<String>>,
    pub inclusive: bool,
}

impl AddressListModule {
    pub fn is_authorized(self, deps: &DepsMut, address: String) -> StdResult<bool> {
        let contract_addr = self.get_contract_address(deps.storage);
        require(
            contract_addr.is_some(),
            StdError::generic_err("Address list does not have an assigned contract address"),
        )?;

        let includes_address =
            query_includes_address(deps.querier, contract_addr.unwrap(), address.clone())?;
        require(
            includes_address == self.inclusive,
            StdError::generic_err("Address is not authorized"),
        )?;

        Ok(true)
    }
}

impl Module for AddressListModule {
    fn validate(&self, all_modules: Vec<ModuleDefinition>) -> StdResult<bool> {
        require(
            is_unique(self, &all_modules),
            StdError::generic_err("Any address list module must be unique"),
        )?;

        //Test to see if the opposite address list type is present
        let opposite_module = AddressListModule {
            address: self.address.clone(),
            code_id: self.code_id.clone(),
            moderators: self.moderators.clone(),
            inclusive: !self.inclusive,
        };
        let mut includes_opposite = all_modules.clone();
        includes_opposite.append(&mut vec![opposite_module.as_definition()]);

        require(
            is_unique(&opposite_module, &includes_opposite),
            StdError::generic_err("Any address list module must be unique"),
        )?;

        require(
            self.address.is_some() || (self.code_id.is_some() && self.moderators.is_some()),
            StdError::generic_err("Address list must include either a contract address or a code id and moderator list"),
        )?;

        Ok(true)
    }
    fn as_definition(&self) -> ModuleDefinition {
        match self.inclusive {
            true => ModuleDefinition::Whitelist {
                address: self.address.clone(),
                code_id: self.code_id.clone(),
                moderators: self.moderators.clone(),
            },
            false => ModuleDefinition::Blacklist {
                address: self.address.clone(),
                code_id: self.code_id.clone(),
                moderators: self.moderators.clone(),
            },
        }
    }
    fn get_contract_address(&self, storage: &dyn Storage) -> Option<String> {
        if self.address.clone().is_some() {
            return Some(self.address.clone().unwrap());
        }
        ADDRESS_LIST_CONTRACT.may_load(storage).unwrap()
    }
}

impl MessageHooks for AddressListModule {
    fn on_instantiate(
        &self,
        _deps: &DepsMut,
        info: MessageInfo,
        _env: Env,
    ) -> StdResult<HookResponse> {
        let mut res = HookResponse::default();
        if self.address.is_none() {
            let inst_msg = WasmMsg::Instantiate {
                admin: Some(info.sender.to_string()),
                code_id: self.code_id.unwrap(),
                funds: vec![],
                label: String::from("Address list instantiation"),
                msg: to_binary(&AddressListInstantiateMsg {
                    moderators: self.moderators.clone().unwrap(),
                })?,
            };

            let msg = SubMsg {
                msg: inst_msg.into(),
                gas_limit: None,
                id: REPLY_ADDRESS_LIST,
                reply_on: ReplyOn::Always,
            };

            res = res.add_message(msg);
        }

        Ok(res)
    }
    fn on_execute(&self, deps: &DepsMut, info: MessageInfo, _env: Env) -> StdResult<HookResponse> {
        self.clone().is_authorized(deps, info.sender.to_string())?;

        Ok(HookResponse::default())
    }
}

pub fn get_address_list_module(storage: &dyn Storage) -> StdResult<ModuleDefinition> {
    let modules = read_modules(storage)?;
    let address_list_def = modules
        .module_defs
        .iter()
        .find(|m| match m {
            ModuleDefinition::Whitelist { .. } => true,
            ModuleDefinition::Blacklist { .. } => true,
            _ => false,
        })
        .ok_or(StdError::generic_err(
            "Token does not implement any address list module",
        ))?;

    Ok(address_list_def.clone())
}

pub fn get_address_list_module_index(storage: &dyn Storage) -> StdResult<usize> {
    let modules = read_modules(storage)?;
    let address_list_def = modules
        .module_defs
        .iter()
        .position(|m| match m {
            ModuleDefinition::Whitelist { .. } => true,
            ModuleDefinition::Blacklist { .. } => true,
            _ => false,
        })
        .ok_or(StdError::generic_err(
            "Token does not implement any address list module",
        ))?;

    Ok(address_list_def.clone())
}

pub fn on_address_list_reply(deps: DepsMut, msg: Reply) -> StdResult<Response> {
    let contract_addr = get_reply_address(msg)?;

    ADDRESS_LIST_CONTRACT.save(deps.storage, &contract_addr.to_string())?;

    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use crate::modules::Rate;

    use super::*;
    // use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockQuerier};

    #[test]
    fn test_validate() {
        let al = AddressListModule {
            moderators: Some(vec![]),
            address: None,
            code_id: Some(1),
            inclusive: true,
        };
        let mut modules = vec![
            al.as_definition().clone(),
            ModuleDefinition::Taxable {
                rate: Rate::Percent(2),
                receivers: vec![],
                description: None,
            },
        ];

        assert_eq!(al.validate(modules.to_vec()), Ok(true));

        modules.push(ModuleDefinition::Whitelist {
            moderators: Some(vec![]),
            address: None,
            code_id: None,
        });

        assert_eq!(
            al.validate(modules.to_vec()),
            Err(StdError::generic_err(
                "Any address list module must be unique"
            ))
        );

        let modules = vec![
            al.as_definition().clone(),
            ModuleDefinition::Taxable {
                rate: Rate::Percent(2),
                receivers: vec![],
                description: None,
            },
            ModuleDefinition::Blacklist {
                moderators: Some(vec![]),
                address: None,
                code_id: None,
            },
        ];

        assert_eq!(
            al.validate(modules.to_vec()),
            Err(StdError::generic_err(
                "Any address list module must be unique"
            ))
        );
    }

    //TODO
    // #[test]
    // fn test_on_execute() {
    //     let sender = "seender";
    //     let mut deps = mock_dependencies(&[]);
    //     deps.querier = deps.querier.with_custom_handler(handler)
    //     let env = mock_env();
    //     let info = mock_info(sender, &[]);
    //     let wl = AddressListModule {
    //         moderators: Some(vec![]),
    //         address: Some(String::from("someaddress")),
    //         code_id: None,
    //         inclusive: true,
    //     };
    //     let msg = ExecuteMsg::Revoke {
    //         spender: String::default(),
    //         token_id: String::default(),
    //     };

    //     let resp = wl
    //         .on_execute(&deps.as_mut(), info.clone(), env.clone(), msg.clone())
    //         .unwrap_err();

    //     assert_eq!(
    //         resp,
    //         StdError::generic_err("Address is not included in the address list")
    //     );

    //     let resp = wl
    //         .on_execute(&deps.as_mut(), info.clone(), env.clone(), msg.clone())
    //         .unwrap();

    //     assert_eq!(resp, HookResponse::default());
    // }
}
