#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, Uint128,
};

use andromeda_protocol::{
    communication::{
        hooks::AndromedaHook,
        modules::{module_hook, register_module, MODULE_ADDR, MODULE_INFO},
    },
    cw20::{ExecuteMsg, InstantiateMsg, QueryMsg},
    error::ContractError,
    require,
    response::get_reply_address,
};
use cw20_base::contract::{
    execute as execute_cw20, execute_burn as execute_cw20_burn, execute_mint as execute_cw20_mint,
    execute_send as execute_cw20_send, execute_transfer as execute_cw20_transfer,
    query as query_cw20,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // How can we use cw20_instantiate without borrowing `deps`? Do we need to replicate the functionality manually?
    // let mut resp = cw20_instantiate(deps, env, info, msg.clone().into())?;
    let mut resp = Response::default();
    if let Some(modules) = msg.modules {
        for module in modules {
            let idx = register_module(deps.storage, deps.api, &module)?;
            if let Some(inst_msg) = module.generate_instantiate_msg(deps.querier, idx)? {
                resp = resp.add_submessage(inst_msg);
            }
        }
    }
    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    require(
        MODULE_INFO.load(deps.storage, msg.id.to_string()).is_ok(),
        ContractError::InvalidReplyId {},
    )?;

    let addr = get_reply_address(&msg)?;
    MODULE_ADDR.save(
        deps.storage,
        msg.id.to_string(),
        &deps.api.addr_validate(&addr)?,
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    module_hook::<Response>(
        deps.storage,
        deps.querier,
        AndromedaHook::OnExecute {
            sender: info.sender.to_string(),
            msg: to_binary(&msg)?,
        },
    )?;
    match msg {
        ExecuteMsg::Transfer { recipient, amount } => {
            execute_transfer(deps, env, info, recipient, amount)
        }
        ExecuteMsg::Burn { amount } => execute_burn(deps, env, info, amount),
        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => execute_send(deps, env, info, contract, amount, msg),
        ExecuteMsg::Mint { recipient, amount } => execute_mint(deps, env, info, recipient, amount),
        _ => Ok(execute_cw20(deps, env, info, msg.into())?),
    }
}

fn execute_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    Ok(execute_cw20_transfer(deps, env, info, recipient, amount)?)
}

fn execute_burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    Ok(execute_cw20_burn(deps, env, info, amount)?)
}

fn execute_send(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {
    Ok(execute_cw20_send(deps, env, info, contract, amount, msg)?)
}

fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    Ok(execute_cw20_mint(deps, env, info, recipient, amount)?)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    Ok(query_cw20(deps, env, msg.into())?)
}
