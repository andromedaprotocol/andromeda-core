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
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub default_yield_strategy: YieldStrategy,
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
    pub default_yield_strategy: StrategyType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
