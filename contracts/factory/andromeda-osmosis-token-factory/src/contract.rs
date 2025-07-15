use andromeda_factory::osmosis_token_factory::{
    ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveHook,
    LockedResponse, FactoryDenomResponse, AllLockedResponse, LockedToken
};

use andromeda_std::{
    ado_contract::ADOContract,
    common::context::ExecuteContext,
    error::ContractError,
    ado_base::InstantiateMsg as BaseInstantiateMsg,
};

use cosmwasm_std::{
    ensure, entry_point, from_json, to_json_binary, wasm_execute, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, QueryRequest, Reply, Response, StdResult, SubMsg, Uint128, WasmQuery
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, Cw20ReceiveMsg, TokenInfoResponse};
use osmosis_std::types::osmosis::tokenfactory::v1beta1::{MsgBurn, MsgCreateDenom, MsgMint};
use osmosis_std::types::cosmos::base::v1beta1::Coin;

use crate::state::{LOCKED, FACTORY_DENOMS, CREATE_DENOM_REPLY_ID, MINT_REPLY_ID, PENDING_MINT};

// Version info for migration
const CONTRACT_NAME: &str = "crates.io:andromeda-osmosis-token-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let inst_resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        &deps.querier,
        info.clone(),
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address.clone(),
            owner: msg.owner,
        },
    )?;
    
    Ok(inst_resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    ctx: ExecuteContext,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute_receive(ctx, msg),
        ExecuteMsg::Unlock { cw20_addr, factory_denom, amount } => {
            execute_unlock(ctx, cw20_addr, factory_denom, amount)
        }
        _ => ADOContract::default().execute(ctx, msg),
    }
}

fn execute_receive(
    ctx: ExecuteContext,
    msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let hook: ReceiveHook = from_json(&msg.msg)?;
    
    match hook {
        ReceiveHook::Lock {} => {
            let cw20_addr = ctx.info.sender.clone();
            let user_addr = ctx.deps.api.addr_validate(&msg.sender)?;
            let amount = msg.amount;
            
            execute_lock(ctx, user_addr, cw20_addr, amount)
        },

    }
}

fn execute_lock(
    ctx: ExecuteContext,
    user_addr: Addr,
    cw20_addr: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {

    // Update locked amount for this (user, cw20_token) pair
    LOCKED.update(
        ctx.deps.storage,
        (user_addr.clone(), cw20_addr.clone()),
        |existing| -> Result<Uint128, ContractError> {
            Ok(existing.unwrap_or_default() + amount)
        }
    )?;

    let token_info: TokenInfoResponse =
        ctx.deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: cw20_addr.to_string(),
            msg: to_json_binary(&Cw20QueryMsg::TokenInfo {})?,
        }))?;
    
    // Check if factory denom exists for this CW20
    let factory_denom = FACTORY_DENOMS.may_load(ctx.deps.storage, cw20_addr.clone())?;
    match factory_denom {
        Some(denom) => {
            // Denom exists, mint directly
            execute_mint_factory_tokens(ctx, user_addr, denom, amount)
        }
        None => {
            // Create new denom first
            execute_create_denom_and_mint(ctx, user_addr, cw20_addr, amount, token_info.name)
        }
    }
}

fn execute_create_denom_and_mint(
    ctx: ExecuteContext,
    user_addr: Addr,
    cw20_addr: Addr,
    amount: Uint128,
    name: String
) -> Result<Response, ContractError> {

    let create_denom_msg = MsgCreateDenom {
        sender: ctx.env.contract.address.to_string(),
        subdenom: name.clone(),
    };
    
    // Store pending mint info for reply
    PENDING_MINT.save(ctx.deps.storage, &(user_addr.clone(), amount, cw20_addr.clone(), name.clone()))?;
    
    let sub_msg = SubMsg::reply_on_success(
        create_denom_msg,
        CREATE_DENOM_REPLY_ID
    );
    
    Ok(Response::new()
        .add_submessage(sub_msg)
        .add_attribute("action", "create_denom_and_mint")
        .add_attribute("cw20_addr", cw20_addr.to_string())
        .add_attribute("name", name))
}

fn execute_mint_factory_tokens(
    ctx: ExecuteContext,
    user_addr: Addr,
    name: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let mint_msg = MsgMint {
        sender: ctx.env.contract.address.to_string(),
        amount: Some(Coin {
            denom: name.clone(),
            amount: amount.to_string(),
        }),
        mint_to_address: user_addr.to_string(),
    };
    
    let sub_msg = SubMsg::reply_on_success(
        mint_msg,
        MINT_REPLY_ID
    );
    
    Ok(Response::new()
        .add_submessage(sub_msg)
        .add_attribute("action", "mint_factory_tokens")
        .add_attribute("recipient", user_addr.to_string())
        .add_attribute("amount", amount.to_string()))
}

fn execute_unlock(
    ctx: ExecuteContext,
    cw20_addr: Addr,
    factory_denom: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let user_addr = ctx.info.sender.clone();
    
    // 1. Check that user has enough locked tokens
    let locked_amount = LOCKED.load(ctx.deps.storage, (user_addr.clone(), cw20_addr.clone()))?;
    ensure!(
        locked_amount >= amount,
        ContractError::InsufficientFunds {}
    );
    
    // 2. Update LOCKED state (subtract unlocked amount)
    LOCKED.update(
        ctx.deps.storage,
        (user_addr.clone(), cw20_addr.clone()),
        |existing| -> Result<Uint128, ContractError> {
            Ok(existing.unwrap_or_default() - amount)  // Safe since we checked above
        }
    )?;
    
    // 3. Burn factory tokens from user's account
    let burn_msg = MsgBurn {
        sender: ctx.env.contract.address.to_string(),
        amount: Some(Coin {
            denom: factory_denom.clone(),
            amount: amount.to_string(),
        }),
        burn_from_address: user_addr.to_string(),
    };
    
    // 4. Send CW20 tokens back to user
    let transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: user_addr.to_string(),
        amount,
    };
    let resp = wasm_execute(cw20_addr.to_string(), &to_json_binary(&transfer_msg)?, vec![])?;
 
    
    Ok(Response::new()
        .add_message(burn_msg)
        .add_message(resp)
        .add_attribute("action", "unlock")
        .add_attribute("user", user_addr.to_string())
        .add_attribute("cw20_addr", cw20_addr.to_string())
        .add_attribute("factory_denom", factory_denom)
        .add_attribute("amount", amount.to_string()))
}

fn handle_create_denom_reply(deps: DepsMut, env: Env, _msg: Reply) -> Result<Response, ContractError> {
    let (user_addr, amount, cw20_addr, name) = PENDING_MINT.load(deps.storage)?;
    PENDING_MINT.remove(deps.storage);
    
    FACTORY_DENOMS.save(deps.storage, cw20_addr, &name)?;

    let mint_msg = MsgMint {
        sender: env.contract.address.to_string(),
        amount: Some(Coin {
            denom: name,
            amount: amount.to_string(),
        }),
        mint_to_address: user_addr.to_string(),
    };
    
    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(mint_msg, MINT_REPLY_ID))
        .add_attribute("action", "denom_created_and_minting"))
}

fn handle_mint_reply(
    _deps: DepsMut,
    _env: Env,
    _msg: Reply,
) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("action", "mint_completed"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        CREATE_DENOM_REPLY_ID => handle_create_denom_reply(deps, env, msg),
        MINT_REPLY_ID => handle_mint_reply(deps, env, msg),
        _ => Err(ContractError::InvalidReplyId {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Locked { owner, cw20_addr } => to_json_binary(&query_locked(deps, owner, cw20_addr)?),
        QueryMsg::FactoryDenom { cw20_addr } => to_json_binary(&query_factory_denom(deps, cw20_addr)?),
        QueryMsg::AllLocked { owner } => to_json_binary(&query_all_locked(deps, owner)?),
        _ => Err(cosmwasm_std::StdError::generic_err("Unsupported query")),
    }
}

fn query_locked(deps: Deps, owner: Addr, cw20_addr: Addr) -> StdResult<LockedResponse> {
    let amount = LOCKED.may_load(deps.storage, (owner, cw20_addr))?.unwrap_or_default();
    Ok(LockedResponse { amount })
}

fn query_factory_denom(deps: Deps, cw20_addr: Addr) -> StdResult<FactoryDenomResponse> {
    let factory_denom = FACTORY_DENOMS.may_load(deps.storage, cw20_addr)?;
    Ok(FactoryDenomResponse { factory_denom })
}

fn query_all_locked(deps: Deps, owner: Addr) -> StdResult<AllLockedResponse> {
    let locked: StdResult<Vec<_>> = LOCKED
        .prefix(owner)
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| {
            let (cw20_addr, amount) = item?;
            let factory_denom = FACTORY_DENOMS.load(deps.storage, cw20_addr.clone())?;
            Ok(LockedToken {
                cw20_addr,
                amount,
                factory_denom,
            })
        })
        .collect();
    
    Ok(AllLockedResponse { locked: locked? })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: Empty) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_VERSION, CONTRACT_NAME)
} 