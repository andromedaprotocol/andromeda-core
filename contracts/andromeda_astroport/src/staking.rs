use andromeda_protocol::{error::ContractError, require};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};

pub fn execute_stake_lp(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lp_token_contract: String,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    Ok(Response::new())
}

pub fn execute_unstake_lp(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lp_token_contract: String,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    Ok(Response::new())
}

pub fn execute_claim_lp_staking_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    auto_stake: Option<bool>,
) -> Result<Response, ContractError> {
    Ok(Response::new())
}

pub fn execute_stake_astro(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    Ok(Response::new())
}

pub fn execute_unstake_astro(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    Ok(Response::new())
}
