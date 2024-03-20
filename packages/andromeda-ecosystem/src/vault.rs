use andromeda_std::amp::recipient::Recipient;
use andromeda_std::amp::AndrAddr;
use andromeda_std::{ado_base::withdraw::Withdrawal, error::ContractError};
use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_json_binary, wasm_execute, Binary, Coin, CosmosMsg, ReplyOn, Storage, SubMsg, Uint128,
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
    pub address: AndrAddr,
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
                    &ExecuteMsg::Deposit {
                        recipient: Some(recipient.address),
                        msg: recipient.msg,
                    },
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
#[derive(Default)]
pub struct DepositMsg {
    pub strategy: Option<StrategyType>,
    pub amount: Option<Coin>,
    pub deposit_msg: Option<Binary>,
}

impl DepositMsg {
    pub fn to_json_binary(&self) -> Result<Binary, ContractError> {
        Ok(to_json_binary(self)?)
    }

    pub fn with_amount(&mut self, amount: Coin) -> &mut Self {
        self.amount = Some(amount);
        self
    }

    pub fn with_strategy(&mut self, strategy: StrategyType) -> &mut Self {
        self.strategy = Some(strategy);
        self
    }
}

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    WithdrawVault {
        recipient: Option<Recipient>,
        withdrawals: Vec<Withdrawal>,
        strategy: Option<StrategyType>,
    },
    UpdateStrategy {
        strategy: StrategyType,
        address: AndrAddr,
    },
    // Originally was an Andromeda Msg
    Withdraw {
        recipient: Option<Recipient>,
        tokens_to_withdraw: Option<Vec<Withdrawal>>,
    },
    // Originally was an Andromeda Msg
    Deposit {
        recipient: Option<::andromeda_std::amp::AndrAddr>,
        msg: Option<::cosmwasm_std::Binary>,
    },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(cosmwasm_std::Binary)]
    VaultBalance {
        address: AndrAddr,
        strategy: Option<StrategyType>,
        denom: Option<String>,
    },
    #[returns(cosmwasm_std::Binary)]
    StrategyAddress { strategy: StrategyType },
}

#[cw_serde]
pub struct StrategyAddressResponse {
    pub strategy: StrategyType,
    pub address: String,
}

#[cw_serde]
pub struct MigrateMsg {}
