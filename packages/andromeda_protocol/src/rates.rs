use crate::{
    communication::{
        encode_binary,
        hooks::{AndromedaHook, OnFundsTransferResponse},
        AndromedaMsg, AndromedaQuery, Recipient,
    },
    error::ContractError,
    modules::Rate,
};
use cosmwasm_std::{
    BankMsg, Coin, CosmosMsg, QuerierWrapper, QueryRequest, SubMsg, Uint128, WasmQuery,
};
use cw20::Cw20Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum Funds {
    Native(Coin),
    Cw20(Cw20Coin),
}

impl Funds {
    // There is probably a more idiomatic way of doing this with From and Into...
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

/// Gets the amount of tax paid by iterating over the `msgs` and comparing it to the
/// `deducted_amount`. It is assumed that each bank message has a single Coin to send as transfer
/// agreements only accept a single Coin. It is also assumed that the result will always be
/// non-negative.
///
/// # Arguments
///
/// * `msgs` - The vector of submessages containing fund transfers
/// * `deducted_amount` - The amount deducted after applying royalties, any surplus paid is tax
pub fn get_tax_amount(msgs: &[SubMsg], deducted_amount: Uint128) -> Uint128 {
    msgs.iter()
        .map(|msg| {
            if let CosmosMsg::Bank(BankMsg::Send { amount, .. }) = &msg.msg {
                amount[0].amount
            } else {
                Uint128::zero()
            }
        })
        .reduce(|total, amount| total + amount)
        .unwrap_or_else(Uint128::zero)
        - deducted_amount
}

pub fn on_required_payments(
    querier: QuerierWrapper,
    addr: String,
    amount: Funds,
) -> Result<OnFundsTransferResponse, ContractError> {
    let res: OnFundsTransferResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: addr,
        msg: encode_binary(&QueryMsg::AndrQuery(AndromedaQuery::Get(Some(
            encode_binary(&amount)?,
        ))))?,
    }))?;

    Ok(res)
}
