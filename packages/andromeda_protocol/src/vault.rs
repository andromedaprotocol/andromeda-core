use common::{
    ado_base::{recipient::Recipient, AndromedaMsg, AndromedaQuery},
    error::ContractError,
};
use cosmwasm_std::{to_binary, wasm_execute, Coin, CosmosMsg, ReplyOn, Storage, SubMsg, Uint128};
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

//Mapping between (Address, Funds Denom) and the amount
pub const BALANCES: Map<(String, String), Uint128> = Map::new("balances");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum YieldStrategy {
    Anchor { address: String },
    None,
}

impl YieldStrategy {
    pub fn deposit(
        &self,
        storage: &mut dyn Storage,
        funds: Coin,
        recipient: &str,
    ) -> Result<Option<SubMsg>, ContractError> {
        let denom = funds.clone().denom;
        match self {
            YieldStrategy::None => {
                let mut balance = BALANCES
                    .load(storage, (recipient.to_string(), denom.clone()))
                    .unwrap_or(Uint128::zero());
                balance = balance.checked_add(funds.amount)?;

                BALANCES.save(storage, (recipient.to_string(), denom), &balance)?;
                Ok(None)
            }
            YieldStrategy::Anchor { address } => {
                let msg = wasm_execute(
                    address,
                    &ExecuteMsg::AndrReceive(AndromedaMsg::Receive(Some(to_binary(recipient)?))),
                    vec![funds],
                )?;
                let sub_msg = SubMsg {
                    id: 1,
                    msg: CosmosMsg::Wasm(msg),
                    gas_limit: None,
                    reply_on: ReplyOn::Error,
                };

                Ok(Some(sub_msg))
            }
        }
    }
}

impl ToString for YieldStrategy {
    fn to_string(&self) -> String {
        match self {
            YieldStrategy::Anchor { .. } => String::from("anchor"),
            YieldStrategy::None => String::from("none"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub default_yield_strategy: YieldStrategy,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecuteMsg {
    Deposit { recipient: Option<Recipient> },
    AndrReceive(AndromedaMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub default_yield_strategy: YieldStrategy,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
