use cosmwasm_std::{
    to_binary, DepsMut, Env, MessageInfo, QuerierWrapper, QueryRequest, Reply, ReplyOn, Response,
    StdError, StdResult, Storage, SubMsg, WasmMsg, WasmQuery,
};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::response::get_reply_address;
use crate::{
    auction::{
        AuctionStateResponse, InstantiateMsg as AuctionInstantiateMsg, QueryMsg as AuctionQueryMsg,
    },
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
    /// * Must include either a contract address or a combination of a valid code id and an optional vector of moderating addresses
    fn validate(
        &self,
        all_modules: Vec<ModuleDefinition>,
        _querier: &QuerierWrapper,
    ) -> Result<bool, ContractError> {
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
        env: Env,
    ) -> Result<HookResponse, ContractError> {
        let mut res = HookResponse::default();
        if self.address.is_none() {
            let inst_msg = WasmMsg::Instantiate {
                admin: Some(info.sender.to_string()),
                code_id: self.code_id.unwrap(),
                funds: vec![],
                label: String::from("Auction instantiation"),
                msg: to_binary(&AuctionInstantiateMsg {
                    token_addr: env.contract.address.to_string(),
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
        _info: MessageInfo,
        _env: Env,
        _recipient: String,
        token_id: String,
    ) -> Result<HookResponse, ContractError> {
        self.verify_token_not_in_auction(deps, token_id)
    }

    fn on_burn(
        &self,
        deps: &DepsMut,
        _info: MessageInfo,
        _env: Env,
        token_id: String,
    ) -> Result<HookResponse, ContractError> {
        self.verify_token_not_in_auction(deps, token_id)
    }

    fn on_archive(
        &self,
        deps: &DepsMut,
        _info: MessageInfo,
        _env: Env,
        token_id: String,
    ) -> Result<HookResponse, ContractError> {
        self.verify_token_not_in_auction(deps, token_id)
    }
}

impl AuctionModule {
    fn verify_token_not_in_auction(
        &self,
        deps: &DepsMut,
        token_id: String,
    ) -> Result<HookResponse, ContractError> {
        let query_msg = AuctionQueryMsg::LatestAuctionState { token_id };
        let contract_addr = self.get_contract_address(deps.storage).unwrap();
        let response: StdResult<AuctionStateResponse> =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr,
                msg: to_binary(&query_msg)?,
            }));
        // Disallow token transfers while token is in auction.
        if let Ok(response) = response {
            if !response.claimed {
                return Err(ContractError::AuctionNotEnded {});
            }
        }
        Ok(HookResponse::default())
    }
}

/// Used to stored the contract address once the contract is instantiated
pub fn on_auction_reply(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let contract_addr = get_reply_address(&msg)?;

    AUCTION_CONTRACT.save(deps.storage, &contract_addr)?;

    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_env, mock_info};

    use crate::{
        modules::Rate,
        testing::mock_querier::{
            mock_dependencies_custom, MOCK_AUCTION_CONTRACT, MOCK_TOKEN_IN_AUCTION,
        },
    };

    use super::*;

    #[test]
    fn test_validate() {
        let deps = mock_dependencies_custom(&[]);
        let al = AuctionModule {
            operators: Some(vec![]),
            address: None,
            code_id: Some(1),
        };
        let mut modules = vec![
            al.as_definition(),
            ModuleDefinition::Taxable {
                rate: Rate::Percent(2u128.into()),
                receivers: vec![],
                description: None,
            },
        ];

        assert_eq!(
            al.validate(modules.to_vec(), &deps.as_ref().querier),
            Ok(true)
        );

        modules.push(ModuleDefinition::Auction {
            operators: Some(vec![]),
            address: None,
            code_id: None,
        });

        assert_eq!(
            al.validate(modules.to_vec(), &deps.as_ref().querier),
            Err(ContractError::ModuleNotUnique {})
        );
    }

    #[test]
    fn test_on_transfer_token_in_auction() {
        let sender = "seender";
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(sender, &[]);
        let auction_module = AuctionModule {
            operators: Some(vec![]),
            address: Some(MOCK_AUCTION_CONTRACT.to_string()),
            code_id: None,
        };

        let res = auction_module.on_transfer(
            &deps.as_mut(),
            info,
            env,
            "recipient".to_string(),
            MOCK_TOKEN_IN_AUCTION.to_string(),
        );
        assert_eq!(ContractError::AuctionNotEnded {}, res.unwrap_err());
    }

    #[test]
    fn test_on_transfer_token_not_in_auction() {
        let sender = "seender";
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(sender, &[]);
        let auction_module = AuctionModule {
            operators: Some(vec![]),
            address: Some(MOCK_AUCTION_CONTRACT.to_string()),
            code_id: None,
        };

        let res = auction_module.on_transfer(
            &deps.as_mut(),
            info,
            env,
            "recipient".to_string(),
            "token2".to_string(),
        );
        assert_eq!(HookResponse::default(), res.unwrap());
    }

    #[test]
    fn test_on_burn_token_in_auction() {
        let sender = "seender";
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(sender, &[]);
        let auction_module = AuctionModule {
            operators: Some(vec![]),
            address: Some(MOCK_AUCTION_CONTRACT.to_string()),
            code_id: None,
        };

        let res =
            auction_module.on_burn(&deps.as_mut(), info, env, MOCK_TOKEN_IN_AUCTION.to_string());
        assert_eq!(ContractError::AuctionNotEnded {}, res.unwrap_err());
    }

    #[test]
    fn test_on_burn_token_not_in_auction() {
        let sender = "seender";
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(sender, &[]);
        let auction_module = AuctionModule {
            operators: Some(vec![]),
            address: Some(MOCK_AUCTION_CONTRACT.to_string()),
            code_id: None,
        };

        let res = auction_module.on_burn(&deps.as_mut(), info, env, "token2".to_string());
        assert_eq!(HookResponse::default(), res.unwrap());
    }

    #[test]
    fn test_on_archive_token_in_auction() {
        let sender = "seender";
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(sender, &[]);
        let auction_module = AuctionModule {
            operators: Some(vec![]),
            address: Some(MOCK_AUCTION_CONTRACT.to_string()),
            code_id: None,
        };

        let res =
            auction_module.on_archive(&deps.as_mut(), info, env, MOCK_TOKEN_IN_AUCTION.to_string());
        assert_eq!(ContractError::AuctionNotEnded {}, res.unwrap_err());
    }

    #[test]
    fn test_on_archive_token_not_in_auction() {
        let sender = "seender";
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(sender, &[]);
        let auction_module = AuctionModule {
            operators: Some(vec![]),
            address: Some(MOCK_AUCTION_CONTRACT.to_string()),
            code_id: None,
        };

        let res = auction_module.on_archive(&deps.as_mut(), info, env, "token2".to_string());
        assert_eq!(HookResponse::default(), res.unwrap());
    }
}
