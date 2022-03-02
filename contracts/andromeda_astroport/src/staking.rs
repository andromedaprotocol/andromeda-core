use crate::{auth::require_is_authorized, state::CONFIG};
use andromeda_protocol::{
    communication::encode_binary, error::ContractError, swapper::query_token_balance,
};
use astroport::{
    generator::{
        Cw20HookMsg as GeneratorCw20HookMsg, ExecuteMsg as GeneratorExecuteMsg,
        PendingTokenResponse, QueryMsg as GeneratorQueryMsg,
    },
    querier::query_factory_config,
    staking::Cw20HookMsg as StakingCw20HookMsg,
};
use cosmwasm_std::{
    Addr, CosmosMsg, DepsMut, Env, MessageInfo, QuerierWrapper, QueryRequest, Response, Storage,
    Uint128, WasmMsg, WasmQuery,
};
use cw20::Cw20ExecuteMsg;
use std::cmp;

pub fn execute_stake_lp(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lp_token_contract: String,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    require_is_authorized(deps.storage, info.sender.as_str())?;
    let balance = query_token_balance(
        &deps.querier,
        deps.api.addr_validate(&lp_token_contract)?,
        env.contract.address,
    )?;
    let amount = cmp::min(amount.unwrap_or(balance), balance);
    let generator_contract = query_generator_address(&deps.querier, deps.storage)?;

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: lp_token_contract.clone(),
            msg: encode_binary(&Cw20ExecuteMsg::Send {
                contract: generator_contract,
                amount,
                msg: encode_binary(&GeneratorCw20HookMsg::Deposit {})?,
            })?,
            funds: vec![],
        }))
        .add_attribute("action", "stake_lp")
        .add_attribute("amount", amount)
        .add_attribute("lp_token", lp_token_contract))
}

pub fn execute_unstake_lp(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lp_token_contract: String,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    require_is_authorized(deps.storage, info.sender.as_str())?;
    let generator_contract = query_generator_address(&deps.querier, deps.storage)?;
    let lp_token = deps.api.addr_validate(&lp_token_contract)?;
    let amount_staked = query_amount_staked(
        &deps.querier,
        generator_contract.clone(),
        lp_token.clone(),
        env.contract.address,
    )?;

    let amount = cmp::min(amount.unwrap_or(amount_staked), amount_staked);
    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: generator_contract,
            funds: vec![],
            msg: encode_binary(&GeneratorExecuteMsg::Withdraw { amount, lp_token })?,
        }))
        .add_attribute("action", "unstake_lp")
        .add_attribute("amount", amount)
        .add_attribute("lp_token", lp_token_contract))
}

pub fn execute_claim_lp_staking_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lp_token_contract: String,
    auto_stake: Option<bool>,
) -> Result<Response, ContractError> {
    require_is_authorized(deps.storage, info.sender.as_str())?;
    let generator_contract = query_generator_address(&deps.querier, deps.storage)?;
    let lp_token = deps.api.addr_validate(&lp_token_contract)?;
    let lp_unstake_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: generator_contract.clone(),
        funds: vec![],
        msg: encode_binary(&GeneratorExecuteMsg::Withdraw {
            // Astroport auto-withdraws rewards when LP tokens are withdrawn, so we can initiate a withdraw
            // of 0 to get the rewards and leave the LP tokens there.
            amount: Uint128::zero(),
            lp_token: lp_token.clone(),
        })?,
    });
    let pending_reward = query_pending_reward(
        &deps.querier,
        generator_contract,
        lp_token,
        env.contract.address.clone(),
    )?;
    let auto_stake = auto_stake.unwrap_or(false);
    let res = Response::new()
        .add_attribute("action", "claim_lp_staking_rewards")
        .add_message(lp_unstake_msg);
    if auto_stake {
        let stake_res = execute_stake_astro(deps, env, info, Some(pending_reward))?;
        Ok(res
            .add_attributes(stake_res.attributes)
            .add_submessages(stake_res.messages))
    } else {
        Ok(res)
    }
}

pub fn execute_stake_astro(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    stake_or_unstake_astro(deps, env, info, amount, true)
}

pub fn execute_unstake_astro(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    stake_or_unstake_astro(deps, env, info, amount, false)
}

fn stake_or_unstake_astro(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
    stake: bool,
) -> Result<Response, ContractError> {
    require_is_authorized(deps.storage, info.sender.as_str())?;

    let config = CONFIG.load(deps.storage)?;
    let (token_addr, msg, action) = if stake {
        (
            config.astro_token_contract,
            StakingCw20HookMsg::Enter {},
            "stake_astro",
        )
    } else {
        (
            config.xastro_token_contract,
            StakingCw20HookMsg::Leave {},
            "unstake_astro",
        )
    };

    let balance = query_token_balance(&deps.querier, token_addr.clone(), env.contract.address)?;
    let amount = cmp::min(amount.unwrap_or(balance), balance);

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: token_addr.to_string(),
            msg: encode_binary(&Cw20ExecuteMsg::Send {
                contract: config.astroport_staking_contract.to_string(),
                amount,
                msg: encode_binary(&msg)?,
            })?,
            funds: vec![],
        }))
        .add_attribute("action", action)
        .add_attribute("amount", amount))
}

fn query_generator_address(
    querier: &QuerierWrapper,
    storage: &dyn Storage,
) -> Result<String, ContractError> {
    let config = CONFIG.load(storage)?;
    let generator_contract =
        query_factory_config(&querier, config.astroport_factory_contract)?.generator_address;
    match generator_contract {
        None => Err(ContractError::GeneratorNotSpecified {}),
        Some(generator) => Ok(generator.to_string()),
    }
}

fn query_amount_staked(
    querier: &QuerierWrapper,
    generator_contract: String,
    lp_token: Addr,
    user: Addr,
) -> Result<Uint128, ContractError> {
    Ok(querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: generator_contract,
        msg: encode_binary(&GeneratorQueryMsg::Deposit {
            lp_token: lp_token.clone(),
            user,
        })?,
    }))?)
}

fn query_pending_reward(
    querier: &QuerierWrapper,
    generator_contract: String,
    lp_token: Addr,
    user: Addr,
) -> Result<Uint128, ContractError> {
    let pending_token_response: PendingTokenResponse =
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: generator_contract,
            msg: encode_binary(&GeneratorQueryMsg::PendingToken { lp_token, user })?,
        }))?;
    Ok(pending_token_response.pending)
}
