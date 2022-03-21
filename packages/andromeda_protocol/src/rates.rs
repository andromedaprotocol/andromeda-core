use crate::modules::Rate;
use common::{
    ado_base::{
        hooks::{AndromedaHook, OnFundsTransferResponse},
        recipient::Recipient,
        AndromedaMsg, AndromedaQuery,
    },
    encode_binary,
    error::ContractError,
    Funds,
};
use cosmwasm_std::{
    BankMsg, Coin, CosmosMsg, QuerierWrapper, QueryRequest, SubMsg, Uint128, WasmQuery,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

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
/// difference between the base amount and the amount left over after royalties.
/// It is assumed that each bank message has a single Coin to send as transfer
/// agreements only accept a single Coin. It is also assumed that the result will always be
/// non-negative.
///
/// # Arguments
///
/// * `msgs` - The vector of submessages containing fund transfers
/// * `base_amount` - The amount paid before tax.
/// * `remaining_amount_after_royalties` - The amount remaining of the base_amount after royalties
///                                        are applied
/// Returns the amount of tax necessary to be paid on top of the `base_amount`.
pub fn get_tax_amount(
    msgs: &[SubMsg],
    base_amount: Uint128,
    remaining_amount_after_royalties: Uint128,
) -> Uint128 {
    let deducted_amount = base_amount - remaining_amount_after_royalties;
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
