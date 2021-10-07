use andromeda_protocol::{response::MsgInstantiateContractResponse, token::QueryMsg};
use cosmwasm_std::{
    to_binary, DepsMut, QuerierWrapper, QueryRequest, Reply, Response, StdError, StdResult,
    WasmQuery,
};
use cw721::ContractInfoResponse;
use protobuf::Message;

use crate::state::store_address;

pub fn on_token_creation_reply(deps: DepsMut, msg: Reply) -> StdResult<Response> {
    let data = msg.result.unwrap().data.unwrap();
    let res: MsgInstantiateContractResponse =
        Message::parse_from_bytes(data.as_slice()).map_err(|_| {
            StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
        })?;
    let token_addr = res.get_contract_address();
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
