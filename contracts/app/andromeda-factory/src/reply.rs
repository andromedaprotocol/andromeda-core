use common::{error::ContractError, response::get_reply_address};
use cosmwasm_std::{to_binary, DepsMut, QuerierWrapper, QueryRequest, Reply, Response, WasmQuery};
use cw721::ContractInfoResponse;

use crate::state::store_address;

pub const REPLY_CREATE_TOKEN: u64 = 1;

pub fn on_token_creation_reply(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let token_addr = get_reply_address(msg)?;
    let info = query_token_config(deps.querier, token_addr.to_string())?;

    store_address(deps.storage, info.symbol, &token_addr)?;

    Ok(Response::new())
}

fn query_token_config(
    querier: QuerierWrapper,
    addr: String,
) -> Result<ContractInfoResponse, ContractError> {
    let res: ContractInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: addr,
        msg: to_binary(&"")?,
    }))?;

    Ok(res)
}
