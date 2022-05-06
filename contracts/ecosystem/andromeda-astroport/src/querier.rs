use cosmwasm_std::{Addr, QuerierWrapper, QueryRequest, Uint128, WasmQuery};

use astroport::{
    asset::{Asset as AstroportAsset, PairInfo},
    generator::{PendingTokenResponse, QueryMsg as GeneratorQueryMsg},
    pair::QueryMsg as PairQueryMsg,
};
use common::{encode_binary, error::ContractError};
use cw_asset::Asset;

pub fn query_pair_given_address(
    querier: &QuerierWrapper,
    pair_address: String,
) -> Result<PairInfo, ContractError> {
    query_pair_contract(querier, pair_address, PairQueryMsg::Pair {})
}

pub fn query_pair_share(
    querier: &QuerierWrapper,
    pair_address: String,
    amount: Uint128,
) -> Result<Vec<Asset>, ContractError> {
    Ok(query_pair_contract::<Vec<AstroportAsset>>(
        querier,
        pair_address,
        PairQueryMsg::Share { amount },
    )?
    .iter()
    .map(|a| a.into())
    .collect())
}

pub fn query_pending_reward(
    querier: &QuerierWrapper,
    generator_contract: String,
    lp_token: Addr,
    user: Addr,
) -> Result<Uint128, ContractError> {
    let pending_token_response: PendingTokenResponse =
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: generator_contract,
            msg: encode_binary(&GeneratorQueryMsg::PendingToken { lp_token, user })?,
        }))?;
    Ok(pending_token_response.pending)
}

pub fn query_amount_staked(
    querier: &QuerierWrapper,
    generator_contract: String,
    lp_token: Addr,
    user: Addr,
) -> Result<Uint128, ContractError> {
    Ok(querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: generator_contract,
        msg: encode_binary(&GeneratorQueryMsg::Deposit { lp_token, user })?,
    }))?)
}

fn query_pair_contract<T: serde::de::DeserializeOwned>(
    querier: &QuerierWrapper,
    pair_address: String,
    msg: PairQueryMsg,
) -> Result<T, ContractError> {
    Ok(querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_address,
        msg: encode_binary(&msg)?,
    }))?)
}
