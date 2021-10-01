use andromeda_protocol::modules::whitelist::Whitelist;
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, IsWhitelistedResponse, QueryMsg},
    state::{State, STATE},
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        creator: info.sender.to_string(),
        whitelist: Whitelist {
            moderators: msg.moderators.clone(),
        },
    };

    STATE.save(deps.storage, &state)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Whitelist { address } => execute_whitelist(deps, info, address),
        ExecuteMsg::RemoveWhitelist { address } => execute_remove_whitelist(deps, info, address),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::IsWhitelisted { address } => query_process(deps, &address),
    }
}

fn query_process(deps: Deps, address: &String) -> StdResult<Binary> {
    let state = STATE.load(deps.storage)?;

    to_binary(&IsWhitelistedResponse {
        whitelisted: state.whitelist.is_whitelisted(deps.storage, address)?,
    })
}

fn execute_whitelist(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    if state.whitelist.is_moderator(&info.sender.to_string()) == false {
        return Err(ContractError::Unauthorized {});
    }

    state
        .whitelist
        .whitelist_addr(deps.storage, &address)
        .unwrap();

    STATE.save(deps.storage, &state)?;

    Ok(Response::new())
}

fn execute_remove_whitelist(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if state.whitelist.is_moderator(&info.sender.to_string()) == false {
        return Err(ContractError::Unauthorized {});
    }

    state
        .whitelist
        .remove_whitelist(deps.storage, &address)
        .unwrap();
    STATE.save(deps.storage, &state)?;

    Ok(Response::new())
}