use common::{
    ado_base::{recipient::Recipient, AndromedaMsg, AndromedaQuery},
    error::ContractError,
    require,
    withdraw::Withdrawal,
};
use cosmwasm_std::{to_binary, wasm_execute, Coin, CosmosMsg, ReplyOn, Storage, SubMsg, Uint128};
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

/// Mapping between (Address, Funds Denom) and the amount
pub const BALANCES: Map<(&str, &str), Uint128> = Map::new("balances");
pub const STRATEGY_CONTRACT_ADDRESSES: Map<String, String> =
    Map::new("strategy_contract_addresses");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum StrategyType {
    Anchor,
    // NoStrategy, //Can be used if we wish to add a default strategy
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct YieldStrategy {
    pub strategy_type: StrategyType,
    pub address: String,
}

impl StrategyType {
    pub fn deposit(
        &self,
        storage: &dyn Storage,
        funds: Coin,
        recipient: &str,
    ) -> Result<SubMsg, ContractError> {
        let address = STRATEGY_CONTRACT_ADDRESSES.load(storage, self.to_string());
        match address {
            Err(_) => Err(ContractError::NotImplemented {
                msg: Some(String::from("This strategy is not supported by this vault")),
            }),
            Ok(addr) => {
                let msg = wasm_execute(
                    addr,
                    &ExecuteMsg::AndrReceive(AndromedaMsg::Receive(Some(to_binary(recipient)?))),
                    vec![funds],
                )?;
                let sub_msg = SubMsg {
                    id: 1,
                    msg: CosmosMsg::Wasm(msg),
                    gas_limit: None,
                    reply_on: ReplyOn::Error,
                };

                Ok(sub_msg)
            }
        }
    }
}

impl fmt::Display for StrategyType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StrategyType::Anchor => write!(f, "anchor"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub strategies: Vec<YieldStrategy>,
    pub operators: Option<Vec<String>>,
}

impl InstantiateMsg {
    pub fn validate(&self) -> Result<(), ContractError> {
        let mut strategies = HashSet::new();
        for yield_strategy in self.strategies.to_vec() {
            require(
                !strategies.contains(&yield_strategy.strategy_type.to_string()),
                ContractError::InvalidStrategy {
                    strategy: yield_strategy.strategy_type.to_string(),
                },
            )?;
            strategies.insert(yield_strategy.strategy_type.to_string());
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecuteMsg {
    Deposit {
        recipient: Option<Recipient>,
        amount: Option<Coin>,
        strategy: Option<StrategyType>,
    },
    Withdraw {
        recipient: Option<Recipient>,
        withdrawals: Vec<Withdrawal>,
        strategy: Option<StrategyType>,
    },
    AndrReceive(AndromedaMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    Balance {
        address: String,
        strategy: Option<StrategyType>,
        denom: Option<String>,
    },
    StrategyAddress {
        strategy: StrategyType,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StrategyAddressResponse {
    pub strategy: StrategyType,
    pub address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[cfg(test)]
mod testing {
    use super::*;

    #[test]
    fn test_instantiate_msg_validate() {
        let duplicate_type_instantiate = InstantiateMsg {
            operators: None,
            strategies: vec![
                YieldStrategy {
                    strategy_type: StrategyType::Anchor,
                    address: "terra1abc".to_string(),
                },
                YieldStrategy {
                    strategy_type: StrategyType::Anchor,
                    address: "terra1def".to_string(),
                },
            ],
        };

        let err = duplicate_type_instantiate.validate().unwrap_err();
        assert_eq!(
            ContractError::InvalidStrategy {
                strategy: StrategyType::Anchor.to_string()
            },
            err
        );

        let duplicate_addr_instantiate = InstantiateMsg {
            operators: None,
            strategies: vec![
                YieldStrategy {
                    strategy_type: StrategyType::Anchor,
                    address: "terra1abc".to_string(),
                },
                YieldStrategy {
                    strategy_type: StrategyType::Anchor,
                    address: "terra1abc".to_string(),
                },
            ],
        };

        let err = duplicate_addr_instantiate.validate().unwrap_err();
        assert_eq!(
            ContractError::InvalidStrategy {
                strategy: StrategyType::Anchor.to_string()
            },
            err
        );

        let valid_instantiate = InstantiateMsg {
            operators: None,
            strategies: vec![YieldStrategy {
                strategy_type: StrategyType::Anchor,
                address: "terra1abc".to_string(),
            }],
        };

        assert!(valid_instantiate.validate().is_ok());
    }
}
