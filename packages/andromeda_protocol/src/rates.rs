use crate::{
    communication::{hooks::AndromedaHook, AndromedaMsg, AndromedaQuery, Recipient},
    error::ContractError,
    modules::Rate,
};
use cosmwasm_std::{to_binary, Coin, QuerierWrapper, QueryRequest, SubMsg, WasmQuery};
use cw20::Cw20Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum Funds {
    Native(Coin),
    Cw20(Cw20Coin),
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
pub struct DeductedFundsResponse {
    pub msgs: Vec<SubMsg>,
    pub leftover_funds: Funds,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RateInfo {
    pub rate: Rate,
    pub is_additive: bool,
    pub description: Option<String>,
    pub receivers: Vec<Recipient>,
}

pub fn on_required_payments(
    querier: QuerierWrapper,
    addr: String,
    amount: Funds,
) -> Result<DeductedFundsResponse, ContractError> {
    let res: DeductedFundsResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: addr,
        msg: to_binary(&QueryMsg::AndrQuery(AndromedaQuery::Get(Some(to_binary(
            &amount,
        )?))))?,
    }))?;

    Ok(res)
}
