use crate::{
    communication::{
        hooks::{AndromedaHook, OnFundsTransferResponse},
        AndromedaMsg, AndromedaQuery, Recipient,
    },
    error::ContractError,
    modules::Rate,
};
use cosmwasm_std::{to_binary, Coin, QuerierWrapper, QueryRequest, Uint128, WasmQuery};
use cw20::Cw20Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum Funds {
    Native(Coin),
    Cw20(Cw20Coin),
}

impl Funds {
    pub fn get_amount(&self) -> Uint128 {
        match self {
            Funds::Native(coin) => coin.amount,
            Funds::Cw20(coin) => coin.amount,
        }
    }

    pub fn try_get_coin(&self) -> Result<Coin, ContractError> {
        match self {
            Funds::Native(coin) => Ok(coin.clone()),
            Funds::Cw20(_) => Err(ContractError::ParsingError {
                err: "Funds is not of type Native".to_string(),
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub rates: Vec<RateInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    UpdateRates { rates: Vec<RateInfo> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    AndrHook(AndromedaHook),
    Payments {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PaymentsResponse {
    pub payments: Vec<RateInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RateInfo {
    pub rate: Rate,
    pub is_additive: bool,
    pub description: Option<String>,
    pub receivers: Vec<Recipient>,
}

/// An attribute struct used for any events that involve a payment
pub struct PaymentAttribute {
    /// The amount paid
    pub amount: Coin,
    /// The address the payment was made to
    pub receiver: String,
}

impl ToString for PaymentAttribute {
    fn to_string(&self) -> String {
        format!("{}<{}", self.receiver, self.amount)
    }
}

pub fn on_required_payments(
    querier: QuerierWrapper,
    addr: String,
    amount: Funds,
) -> Result<OnFundsTransferResponse, ContractError> {
    let res: OnFundsTransferResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: addr,
        msg: to_binary(&QueryMsg::AndrQuery(AndromedaQuery::Get(Some(to_binary(
            &amount,
        )?))))?,
    }))?;

    Ok(res)
}
