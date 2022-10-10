use common::{
    ado_base::{recipient::Recipient, AndromedaMsg, AndromedaQuery},
    app::AndrAddress,
    error::ContractError,
    withdraw::Withdrawal,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_binary, wasm_execute, Binary, Coin, CosmosMsg, ReplyOn, Storage, SubMsg, Uint128,
};
use cw_storage_plus::Map;

use std::fmt;

/// Mapping between (Address, Funds Denom) and the amount
pub const BALANCES: Map<(&str, &str), Uint128> = Map::new("balances");
pub const STRATEGY_CONTRACT_ADDRESSES: Map<String, String> =
    Map::new("strategy_contract_addresses");

#[cw_serde]
pub enum StrategyType {
    Anchor,
    // NoStrategy, //Can be used if we wish to add a default strategy
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct YieldStrategy {
    pub strategy_type: StrategyType,
    pub address: AndrAddress,
}

impl StrategyType {
    pub fn deposit(
        &self,
        storage: &dyn Storage,
        funds: Coin,
        recipient: Recipient,
    ) -> Result<SubMsg, ContractError> {
        let address = STRATEGY_CONTRACT_ADDRESSES.load(storage, self.to_string());
        match address {
            Err(_) => Err(ContractError::NotImplemented {
                msg: Some(String::from("This strategy is not supported by this vault")),
            }),
            Ok(addr) => {
                let msg = wasm_execute(
                    addr,
                    &ExecuteMsg::AndrReceive(AndromedaMsg::Receive(Some(to_binary(&recipient)?))),
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

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
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
    UpdateStrategy {
        strategy: StrategyType,
        address: AndrAddress,
    },
    AndrReceive(AndromedaMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    #[returns(Binary)]
    Balance {
        address: String,
        strategy: Option<StrategyType>,
        denom: Option<String>,
    },
    #[returns(Binary)]
    StrategyAddress { strategy: StrategyType },
}

#[cw_serde]
pub struct StrategyAddressResponse {
    pub strategy: StrategyType,
    pub address: String,
}

#[cw_serde]
pub struct MigrateMsg {}
