use common::{
    ado_base::{recipient::Recipient, AndromedaMsg, AndromedaQuery},
    error::ContractError,
};
use cosmwasm_std::{to_binary, wasm_execute, Coin, CosmosMsg, ReplyOn, Storage, SubMsg, Uint128};
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Mapping between (Address, Funds Denom) and the amount
pub const BALANCES: Map<(String, String), Uint128> = Map::new("balances");
pub const STRATEGY_CONTRACT_ADDRESSES: Map<String, String> =
    Map::new("strategy_contract_addresses");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum StrategyType {
    Anchor,
    NoStrategy,
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

impl ToString for StrategyType {
    fn to_string(&self) -> String {
        match self {
            StrategyType::Anchor => String::from("anchor"),
            StrategyType::NoStrategy => String::from("none"),
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
        for (idx, yield_strategy) in self.strategies.to_vec().iter().enumerate() {
            if self.strategies.iter().enumerate().any(|(index, strategy)| {
                index != idx
                    && (strategy.strategy_type == yield_strategy.strategy_type
                        || strategy.address == yield_strategy.address)
            }) {
                return Err(ContractError::StrategyNotUnique {
                    strategy: yield_strategy.strategy_type.to_string(),
                });
            }

            if yield_strategy.strategy_type == StrategyType::NoStrategy {
                return Err(ContractError::InvalidStrategy {
                    strategy: StrategyType::NoStrategy.to_string(),
                });
            }
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
    AndrReceive(AndromedaMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub operators: Vec<String>,
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
            ContractError::StrategyNotUnique {
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
                    strategy_type: StrategyType::NoStrategy,
                    address: "terra1abc".to_string(),
                },
            ],
        };

        let err = duplicate_addr_instantiate.validate().unwrap_err();
        assert_eq!(
            ContractError::StrategyNotUnique {
                strategy: StrategyType::Anchor.to_string()
            },
            err
        );

        let invalid_strategy_instantiate = InstantiateMsg {
            operators: None,
            strategies: vec![YieldStrategy {
                strategy_type: StrategyType::NoStrategy,
                address: "terra1abc".to_string(),
            }],
        };

        let err = invalid_strategy_instantiate.validate().unwrap_err();
        assert_eq!(
            ContractError::InvalidStrategy {
                strategy: StrategyType::NoStrategy.to_string()
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
