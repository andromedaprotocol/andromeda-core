#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, Storage, Uint128, WasmMsg,
};

use andromeda_protocol::{
    communication::{
        hooks::AndromedaHook,
        modules::{
            execute_alter_module, execute_deregister_module, execute_register_module, module_hook,
            on_funds_transfer, validate_modules, ADOType, MODULE_ADDR, MODULE_INFO,
        },
    },
    cw20::{ExecuteMsg, InstantiateMsg, QueryMsg},
    error::ContractError,
    ownership::CONTRACT_OWNER,
    rates::Funds,
    require,
    response::get_reply_address,
};
use cw721_base::Cw721Contract;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    CONTRACT_OWNER.save(deps.storage, &info.sender)?;
    let mut resp = Response::default();
    let sender = info.sender.as_str();
    if let Some(modules) = msg.modules.clone() {
        validate_modules(&modules, ADOType::CW20)?;
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
    let cw20_resp = cw20_instantiate(deps, env, info, msg.into())?;
    resp = resp
        .add_submessages(cw20_resp.messages)
        .add_attributes(cw20_resp.attributes);

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
        ExecuteMsg::RegisterModule { module } => execute_register_module(
            &deps.querier,
            deps.storage,
            deps.api,
            info.sender.as_str(),
            &module,
            ADOType::CW20,
            true,
        ),
        ExecuteMsg::DeregisterModule { module_idx } => {
            execute_deregister_module(deps, info, module_idx)
        }
        ExecuteMsg::AlterModule { module_idx, module } => {
            execute_alter_module(deps, info, module_idx, &module, ADOType::CW20)
        }
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
    let mut resp = Response::new();
    let sender = info.sender.clone();
    let (payments, events, remainder) = on_funds_transfer(
        deps.storage,
        deps.querier,
        info.sender.to_string(),
        Funds::Cw20(Cw20Coin {
            address: env.contract.address.to_string(),
            amount,
        }),
        to_binary(&ExecuteMsg::Transfer {
            amount,
            recipient: recipient.clone(),
        })?,
    )?;
    let remaining_amount = match remainder {
        Funds::Native(..) => amount, //What do we do in the case that the rates returns remaining amount as native funds?
        Funds::Cw20(coin) => coin.amount,
    };

    // Filter through payment messages to extract cw20 transfer messages to avoid looping
    for msg in payments {
        match msg.msg.clone() {
            // Transfer messages are CosmosMsg::Wasm type
            CosmosMsg::Wasm(wasm_msg) => match wasm_msg {
                WasmMsg::Execute { msg: exec_msg, .. } => {
                    // If binary deserializes to a Cw20ExecuteMsg check the message type
                    if let Ok(transfer_msg) = from_binary::<Cw20ExecuteMsg>(&exec_msg) {
                        match transfer_msg {
                            // If the message is a transfer message then transfer the tokens from the current message sender to the recipient
                            Cw20ExecuteMsg::Transfer { recipient, amount } => {
                                transfer_tokens(
                                    deps.storage,
                                    sender.clone(),
                                    deps.api.addr_validate(&recipient)?,
                                    amount,
                                )?;
                            }
                            // Otherwise add to messages to be sent in response
                            _ => {
                                resp = resp.add_submessage(msg);
                            }
                        }
                    }
                }
                // Otherwise add to messages to be sent in response
                _ => {
                    resp = resp.add_submessage(msg.clone());
                }
            },
            // Otherwise add to messages to be sent in response
            _ => {
                resp = resp.add_submessage(msg);
            }
        }
    }

    // Continue with standard cw20 operation
    let cw20_resp = execute_cw20_transfer(deps, env, info, recipient, remaining_amount)?;
    resp = resp.add_attributes(cw20_resp.attributes).add_events(events);
    Ok(resp)
}

fn transfer_tokens(
    storage: &mut dyn Storage,
    sender: Addr,
    recipient: Addr,
    amount: Uint128,
) -> Result<(), ContractError> {
    BALANCES.update(
        storage,
        &sender,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    BALANCES.update(
        storage,
        &recipient,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;
    Ok(())
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
