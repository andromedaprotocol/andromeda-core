use andromeda_protocol::{
    anchor::{BLunaHubQueryMsg, BLunaHubStateResponse},
    communication::encode_binary,
    error::ContractError,
};
use cosmwasm_std::{QuerierWrapper, QueryRequest, WasmQuery};
use moneymarket::{
    custody::{ConfigResponse as CustodyConfigResponse, QueryMsg as CustodyQueryMsg},
    market::{
        BorrowerInfoResponse, ConfigResponse as MarketConfigResponse, QueryMsg as MarketQueryMsg,
    },
    overseer::{
        CollateralsResponse, ConfigResponse as OverseerConfigResponse, QueryMsg as OverseerQueryMsg,
    },
};

pub fn query_market_config(
    querier: &QuerierWrapper,
    anchor_market: String,
) -> Result<MarketConfigResponse, ContractError> {
    Ok(querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: anchor_market,
        msg: encode_binary(&MarketQueryMsg::Config {})?,
    }))?)
}

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

pub fn query_custody_config(
    querier: &QuerierWrapper,
    anchor_custody: String,
) -> Result<CustodyConfigResponse, ContractError> {
    Ok(querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: anchor_custody,
        msg: encode_binary(&CustodyQueryMsg::Config {})?,
    }))?)
}

pub fn query_overseer_config(
    querier: &QuerierWrapper,
    anchor_overseer: String,
) -> Result<OverseerConfigResponse, ContractError> {
    Ok(querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: anchor_overseer,
        msg: encode_binary(&OverseerQueryMsg::Config {})?,
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

pub fn query_hub_state(
    querier: &QuerierWrapper,
    bluna_hub: String,
) -> Result<BLunaHubStateResponse, ContractError> {
    Ok(querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: bluna_hub,
        msg: encode_binary(&BLunaHubQueryMsg::State {})?,
    }))?)
}
