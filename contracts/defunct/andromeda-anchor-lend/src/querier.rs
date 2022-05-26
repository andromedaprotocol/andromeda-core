use common::{encode_binary, error::ContractError};
use cosmwasm_std::{QuerierWrapper, QueryRequest, WasmQuery};
use moneymarket::{
    market::{BorrowerInfoResponse, QueryMsg as MarketQueryMsg},
    overseer::{CollateralsResponse, QueryMsg as OverseerQueryMsg},
};

pub fn query_borrower_info(
    querier: &QuerierWrapper,
    anchor_market: String,
    borrower: String,
) -> Result<BorrowerInfoResponse, ContractError> {
    Ok(querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: anchor_market,
        msg: encode_binary(&MarketQueryMsg::BorrowerInfo {
            borrower,
            block_height: None,
        })?,
    }))?)
}

pub fn query_collaterals(
    querier: &QuerierWrapper,
    anchor_overseer: String,
    borrower: String,
) -> Result<CollateralsResponse, ContractError> {
    Ok(querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: anchor_overseer,
        msg: encode_binary(&OverseerQueryMsg::Collaterals { borrower })?,
    }))?)
}
