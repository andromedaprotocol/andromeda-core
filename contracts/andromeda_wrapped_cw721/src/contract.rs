use andromeda_protocol::{
    communication::encode_binary,
    error::ContractError,
    ownership::{execute_update_owner, query_contract_owner, CONTRACT_OWNER},
    require,
    wrapped_cw721::{ExecuteMsg, InstantiateMsg, QueryMsg},
};
use cosmwasm_std::{
    attr, coins, entry_point, Addr, BankMsg, Binary, BlockInfo, Coin, CosmosMsg, Deps, DepsMut,
    Env, MessageInfo, QuerierWrapper, QueryRequest, Response, StdResult, Storage, SubMsg, Uint128,
    WasmMsg, WasmQuery,
};
use cw721::{Expiration, OwnerOfResponse};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    Ok(encode_binary(&"asdf".to_string())?)
}
