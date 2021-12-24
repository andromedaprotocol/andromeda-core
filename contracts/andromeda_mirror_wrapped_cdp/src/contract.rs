#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use cw2::set_contract_version;

use crate::state::{Config, CONFIG};
use andromeda_protocol::{
    error::ContractError,
    mirror_wrapped_cdp::{
        ExecuteMsg, InstantiateMsg, MirrorGovExecuteMsg, MirrorMintExecuteMsg,
        MirrorStakingExecuteMsg, QueryMsg,
    },
    ownership::{execute_update_owner, is_contract_owner, query_contract_owner, CONTRACT_OWNER},
    require,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda_mirror_wrapped_cdp";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONTRACT_OWNER.save(deps.storage, &info.sender)?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::MirrorMintExecuteMsg(msg) => execute_mirror_mint_msg(deps, info, msg),
        ExecuteMsg::MirrorStakingExecuteMsg(msg) => execute_mirror_staking_msg(deps, info, msg),
        ExecuteMsg::MirrorGovExecuteMsg(msg) => execute_mirror_gov_msg(deps, info, msg),
        ExecuteMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
        ExecuteMsg::UpdateConfig {
            mirror_mint_contract,
            mirror_staking_contract,
            mirror_gov_contract,
        } => execute_update_config(
            deps,
            info,
            mirror_mint_contract,
            mirror_staking_contract,
            mirror_gov_contract,
        ),
    }
}

pub fn execute_mirror_mint_msg(
    deps: DepsMut,
    info: MessageInfo,
    msg: MirrorMintExecuteMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

pub fn execute_mirror_staking_msg(
    deps: DepsMut,
    info: MessageInfo,
    msg: MirrorStakingExecuteMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

pub fn execute_mirror_gov_msg(
    deps: DepsMut,
    info: MessageInfo,
    msg: MirrorGovExecuteMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    mirror_mint_contract: Option<String>,
    mirror_staking_contract: Option<String>,
    mirror_gov_contract: Option<String>,
) -> Result<Response, ContractError> {
    require(
        is_contract_owner(deps.storage, info.sender.to_string())?,
        ContractError::Unauthorized {},
    )?;
    let mut config = CONFIG.load(deps.storage)?;
    if let Some(mirror_mint_contract) = mirror_mint_contract {
        config.mirror_mint_contract = deps.api.addr_canonicalize(&mirror_mint_contract)?;
    }
    if let Some(mirror_staking_contract) = mirror_staking_contract {
        config.mirror_staking_contract = deps.api.addr_canonicalize(&mirror_staking_contract)?;
    }
    if let Some(mirror_gov_contract) = mirror_gov_contract {
        config.mirror_gov_contract = deps.api.addr_canonicalize(&mirror_gov_contract)?;
    }
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ContractOwner {} => to_binary(&query_contract_owner(deps)?),
        // TODO: Replace panic with actual code.
        _ => panic!(),
    }
}
