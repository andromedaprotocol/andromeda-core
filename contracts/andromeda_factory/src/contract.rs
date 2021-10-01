use andromeda_protocol::{
    factory::{AddressResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    hook::InitHook,
    modules::ModuleDefinition,
    require::require,
    token::InstantiateMsg as TokenInstantiateMsg,
};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    WasmMsg,
};

use crate::state::{
    is_address_defined, is_creator, read_address, read_config, store_address, store_config,
    store_creator, Config,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            token_code_id: msg.token_code_id,
            owner: String::default(),
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Create {
            symbol,
            name,
            modules,
            metadata_limit,
        } => create(deps, env, info, name, symbol, modules, metadata_limit),
        ExecuteMsg::TokenCreationHook { symbol, creator } => {
            token_creation(deps, env, info, symbol, creator)
        }
        ExecuteMsg::UpdateAddress {
            symbol,
            new_address,
        } => update_address(deps, env, info, symbol, new_address),
    }
}

pub fn create(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: String,
    symbol: String,
    modules: Vec<ModuleDefinition>,
    metadata_limit: Option<u64>,
) -> StdResult<Response> {
    let config = read_config(deps.storage)?;

    require(
        !is_address_defined(deps.storage, symbol.to_string())?,
        StdError::generic_err("Symbol is in use"),
    )?;

    let resp = Response::new().add_message(WasmMsg::Instantiate {
        code_id: config.token_code_id,
        funds: vec![],
        label: "".to_string(),
        admin: None,
        msg: to_binary(&TokenInstantiateMsg {
            name: name.to_string(),
            symbol: symbol.to_string(),
            minter: info.sender.to_string(),
            modules,
            init_hook: Some(InitHook {
                msg: to_binary(&ExecuteMsg::TokenCreationHook {
                    symbol: symbol.to_string(),
                    creator: info.sender.to_string(),
                })?,
                contract_addr: info.sender.to_string(),
            }),
            metadata_limit,
        })?,
    });

    Ok(resp)
}

pub fn token_creation(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    symbol: String,
    creator: String,
) -> StdResult<Response> {
    require(
        !is_address_defined(deps.storage, symbol.to_string())?,
        StdError::generic_err("Symbol already has a defined address"),
    )?;

    let address = info.sender.to_string();

    store_address(deps.storage, symbol.to_string(), &address)?;
    store_creator(deps.storage, symbol.to_string(), &creator)?;

    Ok(Response::default())
}

pub fn update_address(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    symbol: String,
    new_address: String,
) -> StdResult<Response> {
    require(
        is_creator(deps.storage, symbol.clone(), info.sender.to_string())?,
        StdError::generic_err("Cannot update address for ADO that you did not create"),
    )?;

    store_address(deps.storage, symbol.clone(), &new_address.clone())?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAddress { symbol } => to_binary(&query_address(deps, symbol)?),
    }
}

fn query_address(deps: Deps, symbol: String) -> StdResult<AddressResponse> {
    let address = read_address(deps.storage, symbol)?;
    Ok(AddressResponse {
        address: address.clone(),
    })
}