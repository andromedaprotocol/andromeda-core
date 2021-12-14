use cosmwasm_std::{
    to_binary, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, StdError, StdResult, Storage,
    SubMsg, WasmMsg,
};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::response::get_reply_address;
use crate::{
    address_list::{query_includes_address, InstantiateMsg as AddressListInstantiateMsg},
    modules::{
        common::is_unique,
        hooks::{HookResponse, MessageHooks},
        {Module, ModuleDefinition},
    },
    require,
};

pub const ADDRESS_LIST_CONTRACT: Item<String> = Item::new("addresslistcontract");
pub const REPLY_ADDRESS_LIST: u64 = 2;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// A struct used to define the Address List module. Can be defined by providing either a contract address or the combination of a code ID and a vector of moderators.
pub struct AddressListModule {
    /// The address of the module contract
    pub address: Option<String>,
    /// The code ID for the module contract
    pub code_id: Option<u64>,
    /// An optional vector of addresses to assign as moderators
    pub moderators: Option<Vec<String>>,
    /// Whether the address list is inclusive, true = whitelist, false = blacklist
    pub inclusive: bool,
}

impl AddressListModule {
    /// Helper function to query the address list contract to determine if the provided address is authorized
    pub fn is_authorized(self, deps: &DepsMut, address: String) -> StdResult<bool> {
        let contract_addr = self.get_contract_address(deps.storage);
        require(
            contract_addr.is_some(),
            StdError::generic_err("Address list does not have an assigned contract address"),
        )?;

        let includes_address =
            query_includes_address(deps.querier, contract_addr.unwrap(), address)?;
        require(
            includes_address == self.inclusive,
            StdError::generic_err("Address is not authorized"),
        )?;

        Ok(true)
    }
}

impl Module for AddressListModule {
    /// Checks the validity of an address list module:
    ///
    /// * Must be unique
    /// * Cannot be included alongside an address list of the opposite type (no mixing whitelist/blacklist)
    /// * Must include either a contract address or a combination of a valid code id and an optional vector of moderating addresses
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
        let mut includes_opposite = all_modules;
        includes_opposite.append(&mut vec![opposite_module.as_definition()]);

        require(
            is_unique(&opposite_module, &includes_opposite),
            StdError::generic_err("An address list module cannot be included alongside an address list module of the opposing type"),
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
                code_id: self.code_id,
                moderators: self.moderators.clone(),
            },
            false => ModuleDefinition::Blacklist {
                address: self.address.clone(),
                code_id: self.code_id,
                moderators: self.moderators.clone(),
            },
        }
    }
    fn get_contract_address(&self, storage: &dyn Storage) -> Option<String> {
        // [GLOBAL-02] Changing is_some() + .unwrap() to if let Some()
        if let Some(address) = &self.address {
            return Some(address.clone());
        }
        ADDRESS_LIST_CONTRACT.may_load(storage).unwrap_or_default()
    }
}

impl MessageHooks for AddressListModule {
    /// Generates an instantiation message for the module contract
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
    /// On any execute message, validates that the sender is authorized by the address list
    fn on_execute(&self, deps: &DepsMut, info: MessageInfo, _env: Env) -> StdResult<HookResponse> {
        self.clone().is_authorized(deps, info.sender.to_string())?;

        Ok(HookResponse::default())
    }
}

/// Used to stored the contract address once the contract is instantiated
pub fn on_address_list_reply(deps: DepsMut, msg: Reply) -> StdResult<Response> {
    let contract_addr = get_reply_address(msg)?;

    ADDRESS_LIST_CONTRACT.save(deps.storage, &contract_addr.to_string())?;

    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_env, mock_info};

    use crate::{modules::Rate, testing::mock_querier::mock_dependencies_custom};

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
            Err(StdError::generic_err("An address list module cannot be included alongside an address list module of the opposing type"))
        );
    }

    //TODO
    #[test]
    fn test_on_execute() {
        let sender = "seender";
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(sender, &[]);
        let invalid_addresslist = AddressListModule {
            moderators: Some(vec![]),
            address: Some(String::from("addresslist_contract_address2")),
            code_id: None,
            inclusive: true,
        };

        let resp = invalid_addresslist
            .on_execute(&deps.as_mut(), info.clone(), env.clone())
            .unwrap_err();

        assert_eq!(resp, StdError::generic_err("Address is not authorized"));

        let valid_addresslist = AddressListModule {
            moderators: Some(vec![]),
            address: Some(String::from("addresslist_contract_address1")),
            code_id: None,
            inclusive: true,
        };

        let resp = valid_addresslist
            .on_execute(&deps.as_mut(), info, env)
            .unwrap();

        assert_eq!(resp, HookResponse::default());
    }
}
