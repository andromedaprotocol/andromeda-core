#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QueryRequest, Response,
    StdResult, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;
use serde::de::DeserializeOwned;

use crate::state::CONFIG;
use andromeda_protocol::{
    error::ContractError,
    mirror_wrapped_cdp::{ExecuteMsg, InstantiateMsg, MirrorMintQueryMsg, QueryMsg},
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
    let config = CONFIG.load(deps.storage)?;
    match msg {
        ExecuteMsg::MirrorMintExecuteMsg(msg) => execute_mirror_msg(
            info,
            config.mirror_mint_contract.to_string(),
            to_binary(&msg)?,
        ),
        ExecuteMsg::MirrorStakingExecuteMsg(msg) => execute_mirror_msg(
            info,
            config.mirror_staking_contract.to_string(),
            to_binary(&msg)?,
        ),
        ExecuteMsg::MirrorGovExecuteMsg(msg) => execute_mirror_msg(
            info,
            config.mirror_gov_contract.to_string(),
            to_binary(&msg)?,
        ),
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

pub fn execute_mirror_msg(
    info: MessageInfo,
    contract_addr: String,
    msg_binary: Binary,
) -> Result<Response, ContractError> {
    let execute_msg = WasmMsg::Execute {
        contract_addr,
        funds: info.funds,
        msg: msg_binary,
    };
    Ok(Response::new().add_messages(vec![CosmosMsg::Wasm(execute_msg)]))
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
        QueryMsg::MirrorMintQueryMsg(msg) => to_binary(&query_mirror_mint(deps, msg)?),
        // TODO: Replace panic with actual code.
        _ => panic!(),
    }
}

pub fn query_mirror_mint(deps: Deps, msg: MirrorMintQueryMsg) -> StdResult<()> {
    let contract_addr = CONFIG.load(deps.storage)?.mirror_mint_contract.to_string();
    match msg {
        MirrorMintQueryMsg::Config {} => query_mirror_msg(deps, contract_addr, to_binary(&msg)?),
        MirrorMintQueryMsg::AssetConfig { .. } => panic!(),
        MirrorMintQueryMsg::Position { .. } => panic!(),
        MirrorMintQueryMsg::Positions { .. } => panic!(),
        MirrorMintQueryMsg::NextPositionIdx {} => panic!(),
    }
}

pub fn query_mirror_msg<T: DeserializeOwned>(
    deps: Deps,
    contract_addr: String,
    msg_binary: Binary,
) -> StdResult<T> {
    let query_msg = WasmQuery::Smart {
        contract_addr,
        msg: msg_binary,
    };
    deps.querier.query(&QueryRequest::Wasm(query_msg))
}
