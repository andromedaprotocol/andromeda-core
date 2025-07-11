use andromeda_fungible_tokens::{
    cw20::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{FactoryInfo, LOCKED_TOKENS},
};
use andromeda_std::{
    ado_base::{AndromedaMsg, AndromedaQuery, InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    amp::AndrAddr,
    andr_execute_fn,
    common::{context::ExecuteContext, encode_binary, Funds},
    error::ContractError,
};
use cosmwasm_std::{
    ensure, from_json, to_json_binary, Addr, Api, Binary, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, StdResult, Storage, SubMsg, Uint128, WasmMsg,
};
use cosmwasm_std::{entry_point, wasm_execute, QueryRequest, Reply, StdError, WasmQuery};

use cw20::{Cw20Coin, Cw20ExecuteMsg, Cw20QueryMsg, TokenInfoResponse};
use cw20_base::{
    contract::{execute as execute_cw20, instantiate as cw20_instantiate, query as cw20_query},
    state::BALANCES,
};

use andromeda_socket::osmosis::ExecuteMsg as SocketExecuteMsg;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-cw20";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Reply IDs
const OSMOSIS_MINT_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let cw20_resp = cw20_instantiate(deps.branch(), env.clone(), info.clone(), msg.clone().into())?;
    let resp = contract.instantiate(
        deps.storage,
        env,
        deps.api,
        &deps.querier,
        info.clone(),
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address.clone(),
            owner: msg.clone().owner,
        },
    )?;

    Ok(resp
        .add_submessages(cw20_resp.messages)
        .add_attributes(cw20_resp.attributes))
}

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    let action = msg.as_ref().to_string();
    match msg {
        ExecuteMsg::Transfer { recipient, amount } => {
            execute_transfer(ctx, recipient, amount, action)
        }
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => execute_transfer_from(ctx, recipient, owner, amount, action),
        ExecuteMsg::Burn { amount } => execute_burn(ctx, amount),
        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => execute_send(ctx, contract, amount, msg, action),
        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => execute_send_from(ctx, contract, amount, msg, action, owner),
        ExecuteMsg::Mint { recipient, amount } => execute_mint(ctx, recipient, amount),
        ExecuteMsg::LockAndMintFactory {
            amount,
            factory_contract,
        } => lock_and_mint_factory(ctx, amount, factory_contract),
        ExecuteMsg::UnlockFromFactory { user, amount } => {
            execute_unlock_from_factory(ctx, user, amount)
        }
        _ => {
            let serialized = encode_binary(&msg)?;
            match from_json::<AndromedaMsg>(&serialized) {
                Ok(msg) => ADOContract::default().execute(ctx, msg),
                _ => Ok(execute_cw20(ctx.deps, ctx.env, ctx.info, msg.into())?),
            }
        }
    }
}

fn lock_and_mint_factory(
    ctx: ExecuteContext,
    amount: Uint128,
    factory_contract: AndrAddr,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps,
        info,
        env,
        contract,
        ..
    } = ctx;

    ensure!(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    ensure_sufficient_available_balance(deps.storage, &info.sender, amount)?;

    let locked_info = FactoryInfo {
        factory_contract: factory_contract.clone(),
        amount: amount,
        user: info.sender.clone(),
    };

    BALANCES.update(deps.storage, &info.sender, |balance| -> StdResult<_> {
        Ok(balance.unwrap_or_default().checked_sub(amount)?)
    })?;

    LOCKED_TOKENS.save(deps.storage, &locked_info)?;

    let factory_address = factory_contract.get_raw_address(&deps.as_ref())?;

    let token_info: TokenInfoResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&Cw20QueryMsg::TokenInfo {})?,
        }))?;

    let msg = SocketExecuteMsg::CreateDenom {
        subdenom: token_info.name.clone(),
        amount,
    };

    let response = wasm_execute(factory_address, &msg, vec![])?;
    let sub_msg = SubMsg::reply_always(response, OSMOSIS_MINT_REPLY_ID);

    Ok(Response::new()
        .add_submessage(sub_msg)
        .add_attribute("action", "lock_and_mint_factory")
        .add_attribute("sender", info.sender.to_string())
        .add_attribute("cw20_tokens_locked", amount)
        .add_attribute("osmosis_subdenom", token_info.name)
        .add_attribute("osmosis_factory_contract", factory_contract.to_string())
        .add_attribute("status", "cw20_locked_osmosis_mint_initiated"))
}

fn get_locked_amount_for_user(
    storage: &dyn cosmwasm_std::Storage,
    user: &Addr,
) -> Result<Uint128, ContractError> {
    let locked_info = LOCKED_TOKENS.may_load(storage)?;
    match locked_info {
        Some(info) if info.user == *user => Ok(info.amount),
        _ => Ok(Uint128::zero()),
    }
}

fn get_available_balance(
    storage: &dyn cosmwasm_std::Storage,
    user: &Addr,
) -> Result<Uint128, ContractError> {
    let total_balance = BALANCES.may_load(storage, user)?.unwrap_or_default();
    let locked_amount = get_locked_amount_for_user(storage, user)?;

    // Available = Total - Locked
    Ok(total_balance.saturating_sub(locked_amount))
}

fn ensure_sufficient_available_balance(
    storage: &dyn cosmwasm_std::Storage,
    user: &Addr,
    required: Uint128,
) -> Result<(), ContractError> {
    let available = get_available_balance(storage, user)?;
    if available < required {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Insufficient available balance. Available: {}, Required: {}, Locked: {}",
            available,
            required,
            get_locked_amount_for_user(storage, user)?
        ))));
    }
    Ok(())
}
fn execute_transfer(
    ctx: ExecuteContext,
    recipient: AndrAddr,
    amount: Uint128,
    action: String,
) -> Result<Response, ContractError> {
    ensure_sufficient_available_balance(ctx.deps.storage, &ctx.info.sender, amount)?;
    handle_transfer(ctx, recipient, None, amount, action, false)
}

fn execute_transfer_from(
    ctx: ExecuteContext,
    recipient: AndrAddr,
    owner: String,
    amount: Uint128,
    action: String,
) -> Result<Response, ContractError> {
    let owner_addr = ctx.deps.api.addr_validate(&owner)?;
    ensure_sufficient_available_balance(ctx.deps.storage, &owner_addr, amount)?;
    handle_transfer(ctx, recipient, Some(owner), amount, action, true)
}

fn handle_transfer(
    ctx: ExecuteContext,
    recipient: AndrAddr,
    owner: Option<String>,
    amount: Uint128,
    action: String,
    is_transfer_from: bool,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;

    let transfer_response = ADOContract::default().query_deducted_funds(
        deps.as_ref(),
        action,
        Funds::Cw20(Cw20Coin {
            address: env.contract.address.to_string(),
            amount,
        }),
    )?;
    match transfer_response {
        Some(transfer_response) => {
            let remaining_amount = match transfer_response.leftover_funds {
                Funds::Native(..) => amount, // Handle the case where remaining amount is native funds
                Funds::Cw20(coin) => coin.amount,
            };

            let mut resp = filter_out_cw20_messages(
                transfer_response.msgs,
                deps.storage,
                deps.api,
                &info.sender,
            )?;

            let recipient = recipient.get_raw_address(&deps.as_ref())?.into_string();
            let cw20_msg = if is_transfer_from {
                Cw20ExecuteMsg::TransferFrom {
                    recipient,
                    owner: owner.ok_or(ContractError::new(
                        "Owner should be provided for TransferFrom",
                    ))?,
                    amount: remaining_amount,
                }
            } else {
                Cw20ExecuteMsg::Transfer {
                    recipient,
                    amount: remaining_amount,
                }
            };

            let cw20_resp = execute_cw20(deps, env, info, cw20_msg)?;
            resp = resp
                .add_submessages(cw20_resp.messages)
                .add_attributes(cw20_resp.attributes)
                .add_events(transfer_response.events);
            Ok(resp)
        }
        None => {
            let recipient = recipient.get_raw_address(&deps.as_ref())?.into_string();
            let cw20_msg = if is_transfer_from {
                Cw20ExecuteMsg::TransferFrom {
                    recipient,
                    owner: owner.ok_or(ContractError::new(
                        "Owner should be provided for TransferFrom",
                    ))?,
                    amount,
                }
            } else {
                Cw20ExecuteMsg::Transfer { recipient, amount }
            };

            let cw20_resp = execute_cw20(deps, env, info, cw20_msg)?;
            Ok(cw20_resp)
        }
    }
}

fn transfer_tokens(
    storage: &mut dyn Storage,
    sender: &Addr,
    recipient: &Addr,
    amount: Uint128,
) -> Result<(), ContractError> {
    BALANCES.update(
        storage,
        sender,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    BALANCES.update(
        storage,
        recipient,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_add(amount)?)
        },
    )?;
    Ok(())
}

fn execute_burn(ctx: ExecuteContext, amount: Uint128) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;
    ensure_sufficient_available_balance(deps.storage, &info.sender, amount)?;
    Ok(execute_cw20(
        deps,
        env,
        info,
        Cw20ExecuteMsg::Burn { amount },
    )?)
}

fn execute_send(
    ctx: ExecuteContext,
    contract: AndrAddr,
    amount: Uint128,
    msg: Binary,
    action: String,
) -> Result<Response, ContractError> {
    ensure_sufficient_available_balance(ctx.deps.storage, &ctx.info.sender, amount)?;
    handle_send(ctx, contract, amount, msg, action, None, false)
}

fn execute_send_from(
    ctx: ExecuteContext,
    contract: AndrAddr,
    amount: Uint128,
    msg: Binary,
    action: String,
    owner: String,
) -> Result<Response, ContractError> {
    let owner_addr = ctx.deps.api.addr_validate(&owner)?;
    ensure_sufficient_available_balance(ctx.deps.storage, &owner_addr, amount)?;
    handle_send(ctx, contract, amount, msg, action, Some(owner), true)
}

fn handle_send(
    ctx: ExecuteContext,
    contract: AndrAddr,
    amount: Uint128,
    msg: Binary,
    action: String,
    owner: Option<String>,
    is_send_from: bool,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;

    let rates_response = ADOContract::default().query_deducted_funds(
        deps.as_ref(),
        action,
        Funds::Cw20(Cw20Coin {
            address: env.contract.address.to_string(),
            amount,
        }),
    )?;
    match rates_response {
        Some(rates_response) => {
            let remaining_amount = match rates_response.leftover_funds {
                Funds::Native(..) => amount, // Handle the case where remaining amount is native funds
                Funds::Cw20(coin) => coin.amount,
            };

            let mut resp = filter_out_cw20_messages(
                rates_response.msgs,
                deps.storage,
                deps.api,
                &info.sender,
            )?;
            let contract = contract.get_raw_address(&deps.as_ref())?.to_string();
            let cw20_msg = if is_send_from {
                Cw20ExecuteMsg::SendFrom {
                    contract,
                    amount: remaining_amount,
                    msg,
                    owner: owner
                        .ok_or(ContractError::new("Owner should be provided for SendFrom"))?,
                }
            } else {
                Cw20ExecuteMsg::Send {
                    contract,
                    amount: remaining_amount,
                    msg,
                }
            };

            let cw20_resp = execute_cw20(deps, env, info, cw20_msg)?;
            resp = resp
                .add_submessages(cw20_resp.messages)
                .add_attributes(cw20_resp.attributes)
                .add_events(rates_response.events);

            Ok(resp)
        }
        None => {
            let contract = contract.get_raw_address(&deps.as_ref())?.to_string();
            let cw20_msg = if is_send_from {
                Cw20ExecuteMsg::SendFrom {
                    contract,
                    amount,
                    msg,
                    owner: owner
                        .ok_or(ContractError::new("Owner should be provided for SendFrom"))?,
                }
            } else {
                Cw20ExecuteMsg::Send {
                    contract,
                    amount,
                    msg,
                }
            };
            let cw20_resp = execute_cw20(deps, env, info, cw20_msg)?;
            Ok(cw20_resp)
        }
    }
}

fn execute_mint(
    ctx: ExecuteContext,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;

    Ok(execute_cw20(
        deps,
        env,
        info,
        Cw20ExecuteMsg::Mint { recipient, amount },
    )?)
}

fn execute_unlock_from_factory(
    ctx: ExecuteContext,
    user: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

    // Verify caller is authorized (only owner for now - you can modify this)
    let contract = ADOContract::default();
    ensure!(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    // Validate user address
    let user_addr = deps.api.addr_validate(&user)?;

    // Check that there are locked tokens for this user
    let locked_info = LOCKED_TOKENS.load(deps.storage)?;
    ensure!(
        locked_info.user == user_addr,
        ContractError::Std(StdError::generic_err("No locked tokens for this user"))
    );

    // Check that unlock amount doesn't exceed locked amount
    ensure!(
        amount <= locked_info.amount,
        ContractError::Std(StdError::generic_err(format!(
            "Cannot unlock {} tokens, only {} are locked",
            amount, locked_info.amount
        )))
    );

    // Restore user's balance
    BALANCES.update(deps.storage, &user_addr, |balance| -> StdResult<_> {
        Ok(balance.unwrap_or_default().checked_add(amount)?)
    })?;

    // Update locked tokens state
    if amount == locked_info.amount {
        // Full unlock - remove locked state
        LOCKED_TOKENS.remove(deps.storage);
    } else {
        // Partial unlock - update locked amount
        let updated_info = FactoryInfo {
            factory_contract: locked_info.factory_contract,
            amount: locked_info.amount.checked_sub(amount)?,
            user: user_addr.clone(),
        };
        LOCKED_TOKENS.save(deps.storage, &updated_info)?;
    }

    Ok(Response::new()
        .add_attribute("action", "unlock_from_factory")
        .add_attribute("user", user_addr)
        .add_attribute("amount_unlocked", amount)
        .add_attribute("caller", info.sender))
}

fn filter_out_cw20_messages(
    msgs: Vec<SubMsg>,
    storage: &mut dyn Storage,
    api: &dyn Api,
    sender: &Addr,
) -> Result<Response, ContractError> {
    let mut resp: Response = Response::new();
    // Filter through payment messages to extract cw20 transfer messages to avoid looping
    for sub_msg in msgs {
        // Transfer messages are CosmosMsg::Wasm type
        if let CosmosMsg::Wasm(WasmMsg::Execute { msg: exec_msg, .. }) = sub_msg.msg.clone() {
            // If binary deserializes to a Cw20ExecuteMsg check the message type
            if let Ok(Cw20ExecuteMsg::Transfer { recipient, amount }) =
                from_json::<Cw20ExecuteMsg>(&exec_msg)
            {
                transfer_tokens(storage, sender, &api.addr_validate(&recipient)?, amount)?;
            } else {
                resp = resp.add_submessage(sub_msg);
            }
        } else {
            resp = resp.add_submessage(sub_msg);
        }
    }
    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let serialized = to_json_binary(&msg)?;
    match from_json::<AndromedaQuery>(&serialized) {
        Ok(msg) => ADOContract::default().query(deps, env, msg),
        _ => Ok(cw20_query(deps, env, msg.into())?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        OSMOSIS_MINT_REPLY_ID => handle_osmosis_mint_reply(msg, _deps),
        _ => {
            if msg.result.is_err() {
                return Err(ContractError::Std(StdError::generic_err(
                    msg.result.unwrap_err(),
                )));
            }
            Ok(Response::default())
        }
    }
}

fn handle_osmosis_mint_reply(msg: Reply, deps: DepsMut) -> Result<Response, ContractError> {
    match msg.result {
        cosmwasm_std::SubMsgResult::Ok(_) => {
            Ok(Response::new().add_attribute("osmosis_mint", "success"))
        }
        cosmwasm_std::SubMsgResult::Err(error) => {
            let balance_to_refund = LOCKED_TOKENS.load(deps.storage)?;
            BALANCES.update(
                deps.storage,
                &balance_to_refund.user,
                |balance| -> StdResult<_> {
                    Ok(balance
                        .unwrap_or_default()
                        .checked_add(balance_to_refund.amount)?)
                },
            )?;
            LOCKED_TOKENS.remove(deps.storage);

            Err(ContractError::Std(StdError::generic_err(error)))
        }
    }
}
