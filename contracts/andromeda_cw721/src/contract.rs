#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response, StdError, Uint128,
};

use andromeda_protocol::{
    communication::{
        hooks::AndromedaHook,
        modules::{
            execute_register_module, module_hook, validate_modules, ADOType, MODULE_ADDR,
            MODULE_INFO,
        },
    },
    cw721::{InstantiateMsg, QueryMsg, TokenExtension},
    error::ContractError,
    ownership::CONTRACT_OWNER,
    require,
    response::get_reply_address,
    token::ExecuteMsg,
};
use cw721_base::Cw721Contract;

pub type AndrCW721Contract<'a> = Cw721Contract<'a, TokenExtension, Empty>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    CONTRACT_OWNER.save(deps.storage, &info.sender)?;

    let sender = info.sender.as_str();
    let mut resp = Response::default();
    if let Some(modules) = msg.modules.clone() {
        validate_modules(&modules, ADOType::CW721)?;
        for module in modules {
            resp = execute_register_module(
                &deps.querier,
                deps.storage,
                deps.api,
                sender,
                &module,
                ADOType::CW20,
                false,
            )?;
        }
    }
    let cw721_resp = AndrCW721Contract::default().instantiate(deps, env, info, msg.into())?;
    resp = resp
        .add_attributes(cw721_resp.attributes)
        .add_submessages(cw721_resp.messages);
    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    let id = msg.id.to_string();
    require(
        MODULE_INFO.load(deps.storage, &id).is_ok(),
        ContractError::InvalidReplyId {},
    )?;

    let addr = get_reply_address(&msg)?;
    MODULE_ADDR.save(deps.storage, &id, &deps.api.addr_validate(&addr)?)?;

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
            payload: to_binary(&msg)?,
        },
    )?;

    Ok(Response::default())
}

fn execute_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    Ok(AndrCW721Contract::default().query(deps, env, msg.into())?)
}
