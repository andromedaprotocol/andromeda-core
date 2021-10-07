use andromeda_protocol::{response::get_reply_address, token::QueryMsg};
use cosmwasm_std::{
    to_binary, DepsMut, QuerierWrapper, QueryRequest, Reply, Response, StdResult, WasmQuery,
};
use cw721::ContractInfoResponse;

use crate::state::store_address;

pub const REPLY_CREATE_TOKEN: u64 = 1;

pub fn on_token_creation_reply(deps: DepsMut, msg: Reply) -> StdResult<Response> {
    let token_addr = get_reply_address(msg)?;
    let info = query_token_config(deps.querier, token_addr.to_string())?;

    store_address(deps.storage, info.symbol, &token_addr.to_string())?;

    Ok(Response::new())
}

fn query_token_config(querier: QuerierWrapper, addr: String) -> StdResult<ContractInfoResponse> {
    let res: ContractInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: addr,
        msg: to_binary(&QueryMsg::ContractInfo {})?,
    }))?;

    Ok(res)
}
