use cosmwasm_std::{
    to_binary, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, StdError, StdResult, Storage,
    SubMsg, WasmMsg,
};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::response::get_reply_address;
use crate::{
    auction::InstantiateMsg as AuctionInstantiateMsg,
    error::ContractError,
    modules::{
        common::is_unique,
        hooks::{HookResponse, MessageHooks},
        {Module, ModuleDefinition},
    },
    require,
};

pub const AUCTION_CONTRACT: Item<String> = Item::new("auctioncontract");
pub const REPLY_AUCTION: u64 = 3;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// A struct used to define the Auction module. Can be defined by providing either a contract address or the combination of a code ID and a vector of operators.
pub struct AuctionModule {
    /// The address of the module contract
    pub address: Option<String>,
    /// The code ID for the module contract
    pub code_id: Option<u64>,
    /// An optional vector of addresses to assign as operators
    pub operators: Option<Vec<String>>,
}

impl Module for AuctionModule {
    /// Checks the validity of an auction module:
    ///
    /// * Must be unique
    /// * Cannot be included alongside an address list of the opposite type (no mixing whitelist/blacklist)
    /// * Must include either a contract address or a combination of a valid code id and an optional vector of moderating addresses
    fn validate(&self, all_modules: Vec<ModuleDefinition>) -> Result<bool, ContractError> {
        require(
            is_unique(self, &all_modules),
            ContractError::ModuleNotUnique {},
        )?;

        require(
            self.address.is_some() || (self.code_id.is_some() && self.operators.is_some()),
            ContractError::Std(StdError::generic_err(
                "Auction must include either a contract address or a code id and operator list",
            )),
        )?;

        Ok(true)
    }
    fn as_definition(&self) -> ModuleDefinition {
        ModuleDefinition::Auction {
            address: self.address.clone(),
            code_id: self.code_id,
            operators: self.operators.clone(),
        }
    }
    fn get_contract_address(&self, storage: &dyn Storage) -> Option<String> {
        // [GLOBAL-02] Changing is_some() + .unwrap() to if let Some()
        if let Some(address) = &self.address {
            return Some(address.clone());
        }
        AUCTION_CONTRACT.may_load(storage).unwrap_or_default()
    }
}

impl MessageHooks for AuctionModule {
    /// Generates an instantiation message for the module contract
    fn on_instantiate(
        &self,
        _deps: &DepsMut,
        info: MessageInfo,
        _env: Env,
    ) -> Result<HookResponse, ContractError> {
        let mut res = HookResponse::default();
        if self.address.is_none() {
            let inst_msg = WasmMsg::Instantiate {
                admin: Some(info.sender.to_string()),
                code_id: self.code_id.unwrap(),
                funds: vec![],
                label: String::from("Auction instantiation"),
                msg: to_binary(&AuctionInstantiateMsg {
                    token_addr: info.sender.to_string(),
                })?,
            };

            let msg = SubMsg {
                msg: inst_msg.into(),
                gas_limit: None,
                id: REPLY_AUCTION,
                reply_on: ReplyOn::Always,
            };

            res = res.add_message(msg);
        }

        Ok(res)
    }

    fn on_transfer(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        recipient: String,
        token_id: String,
    ) -> Result<HookResponse, ContractError> {
        let mut res = HookResponse::default();

        Ok(res)
    }
}

/// Used to stored the contract address once the contract is instantiated
pub fn on_auction_reply(deps: DepsMut, msg: Reply) -> StdResult<Response> {
    let contract_addr = get_reply_address(&msg)?;

    AUCTION_CONTRACT.save(deps.storage, &contract_addr)?;

    Ok(Response::new())
}

/*#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_env, mock_info};

    use crate::{modules::Rate, testing::mock_querier::mock_dependencies_custom};

    use super::*;
    // use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockQuerier};

    #[test]
    fn test_validate() {
        let al = AddressListModule {
            operators: Some(vec![]),
            address: None,
            code_id: Some(1),
            inclusive: true,
        };
        let mut modules = vec![
            al.as_definition(),
            ModuleDefinition::Taxable {
                rate: Rate::Percent(2),
                receivers: vec![],
                description: None,
            },
        ];

        assert_eq!(al.validate(modules.to_vec()), Ok(true));

        modules.push(ModuleDefinition::Whitelist {
            operators: Some(vec![]),
            address: None,
            code_id: None,
        });

        assert_eq!(
            al.validate(modules.to_vec()),
            Err(ContractError::ModuleNotUnique {})
        );

        let modules = vec![
            al.as_definition(),
            ModuleDefinition::Taxable {
                rate: Rate::Percent(2),
                receivers: vec![],
                description: None,
            },
            ModuleDefinition::Blacklist {
                operators: Some(vec![]),
                address: None,
                code_id: None,
            },
        ];

        assert_eq!(
            al.validate(modules.to_vec()),
            Err(ContractError::Std(StdError::generic_err("An address list module cannot be included alongside an address list module of the opposing type"))
        ));
    }

    //TODO
    #[test]
    fn test_on_execute() {
        let sender = "seender";
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(sender, &[]);
        let invalid_addresslist = AddressListModule {
            operators: Some(vec![]),
            address: Some(String::from("addresslist_contract_address2")),
            code_id: None,
            inclusive: true,
        };

        let resp = invalid_addresslist
            .on_execute(&deps.as_mut(), info.clone(), env.clone())
            .unwrap_err();

        assert_eq!(resp, ContractError::Unauthorized {});

        let valid_addresslist = AddressListModule {
            operators: Some(vec![]),
            address: Some(String::from("addresslist_contract_address1")),
            code_id: None,
            inclusive: true,
        };

        let resp = valid_addresslist
            .on_execute(&deps.as_mut(), info, env)
            .unwrap();

        assert_eq!(resp, HookResponse::default());
    }
}*/
