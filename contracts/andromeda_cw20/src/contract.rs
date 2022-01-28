#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, Storage, Uint128, WasmMsg,
};

use andromeda_protocol::{
    communication::{
        hooks::AndromedaHook,
        modules::{module_hook, on_funds_transfer, register_module, MODULE_ADDR, MODULE_INFO},
    },
    cw20::{ExecuteMsg, InstantiateMsg, QueryMsg},
    error::ContractError,
    rates::Funds,
    require,
    response::get_reply_address,
};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use cw20_base::contract::{
    execute as execute_cw20, execute_burn as execute_cw20_burn, execute_mint as execute_cw20_mint,
    execute_send as execute_cw20_send, execute_transfer as execute_cw20_transfer,
    instantiate as cw20_instantiate, query as query_cw20,
};
use cw20_base::state::BALANCES;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let mut resp = Response::default();
    if let Some(modules) = msg.modules.clone() {
        for module in modules {
            let idx = register_module(deps.storage, deps.api, &module)?;
            if let Some(inst_msg) = module.generate_instantiate_msg(deps.querier, idx)? {
                resp = resp.add_submessage(inst_msg);
            }
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
    let mut resp = Response::new();
    let sender = info.sender.clone();
    let (payments, remainder) = on_funds_transfer(
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
    resp = resp.add_attributes(cw20_resp.attributes);
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
