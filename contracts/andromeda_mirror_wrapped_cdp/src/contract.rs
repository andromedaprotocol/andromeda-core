#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128, WasmMsg,
};
use cw2::set_contract_version;

use crate::state::{Config, CONFIG};
use andromeda_protocol::{
    communication::encode_binary,
    error::ContractError,
    mirror_wrapped_cdp::{ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg},
    operators::{execute_update_operators, initialize_operators, is_operator, query_is_operator},
    ownership::{execute_update_owner, is_contract_owner, query_contract_owner, CONTRACT_OWNER},
    require,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use terraswap::asset::{Asset, AssetInfo};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda_mirror_wrapped_cdp";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    if let Some(operators) = msg.operators {
        initialize_operators(deps.storage, operators)?;
    }
    let config = Config {
        mirror_mint_contract: deps.api.addr_validate(&msg.mirror_mint_contract)?,
        mirror_staking_contract: deps.api.addr_validate(&msg.mirror_staking_contract)?,
        mirror_gov_contract: deps.api.addr_validate(&msg.mirror_gov_contract)?,
        mirror_lock_contract: deps.api.addr_validate(&msg.mirror_lock_contract)?,
    };
    CONFIG.save(deps.storage, &config)?;
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
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
        ExecuteMsg::MirrorMintExecuteMsg(msg) => execute_mirror_msg(
            deps,
            info.sender.to_string(),
            info.funds,
            config.mirror_mint_contract.to_string(),
            encode_binary(&msg)?,
        ),
        ExecuteMsg::MirrorStakingExecuteMsg(msg) => execute_mirror_msg(
            deps,
            info.sender.to_string(),
            info.funds,
            config.mirror_staking_contract.to_string(),
            encode_binary(&msg)?,
        ),
        ExecuteMsg::MirrorGovExecuteMsg(msg) => execute_mirror_msg(
            deps,
            info.sender.to_string(),
            info.funds,
            config.mirror_gov_contract.to_string(),
            encode_binary(&msg)?,
        ),
        ExecuteMsg::MirrorLockExecuteMsg(msg) => execute_mirror_msg(
            deps,
            info.sender.to_string(),
            info.funds,
            config.mirror_lock_contract.to_string(),
            encode_binary(&msg)?,
        ),
        ExecuteMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
        ExecuteMsg::UpdateConfig {
            mirror_mint_contract,
            mirror_staking_contract,
            mirror_gov_contract,
            mirror_lock_contract,
        } => execute_update_config(
            deps,
            info,
            mirror_mint_contract,
            mirror_staking_contract,
            mirror_gov_contract,
            mirror_lock_contract,
        ),
        ExecuteMsg::UpdateOperators { operators } => {
            execute_update_operators(deps, info, operators)
        }
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let token_address = info.sender.to_string();
    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::MirrorMintCw20HookMsg(msg) => execute_mirror_cw20_msg(
            deps,
            cw20_msg.sender,
            token_address,
            cw20_msg.amount,
            config.mirror_mint_contract.to_string(),
            encode_binary(&msg)?,
        ),
        Cw20HookMsg::MirrorStakingCw20HookMsg(msg) => execute_mirror_cw20_msg(
            deps,
            cw20_msg.sender,
            token_address,
            cw20_msg.amount,
            config.mirror_staking_contract.to_string(),
            encode_binary(&msg)?,
        ),
        Cw20HookMsg::MirrorGovCw20HookMsg(msg) => execute_mirror_cw20_msg(
            deps,
            cw20_msg.sender,
            token_address,
            cw20_msg.amount,
            config.mirror_gov_contract.to_string(),
            encode_binary(&msg)?,
        ),
    }
}

pub fn execute_mirror_cw20_msg(
    deps: DepsMut,
    sender: String,
    token_addr: String,
    amount: Uint128,
    contract_addr: String,
    msg_binary: Binary,
) -> Result<Response, ContractError> {
    let msg = Cw20ExecuteMsg::Send {
        contract: contract_addr,
        amount,
        msg: msg_binary,
    };
    execute_mirror_msg(deps, sender, vec![], token_addr, encode_binary(&msg)?)
}

pub fn execute_mirror_msg(
    deps: DepsMut,
    sender: String,
    funds: Vec<Coin>,
    contract_addr: String,
    msg_binary: Binary,
) -> Result<Response, ContractError> {
    require(
        is_contract_owner(deps.storage, sender.as_str())?
            || is_operator(deps.storage, sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    require(
        funds.is_empty() || funds.len() == 1,
        ContractError::InvalidMirrorFunds {
            msg: "Mirror expects no funds or a single type of fund to be deposited.".to_string(),
        },
    )?;
    let tax_deducted_funds = get_tax_deducted_funds(&deps, funds)?;

    let execute_msg = WasmMsg::Execute {
        contract_addr,
        funds: tax_deducted_funds,
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
    mirror_lock_contract: Option<String>,
) -> Result<Response, ContractError> {
    require(
        is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    let mut config = CONFIG.load(deps.storage)?;
    if let Some(mirror_mint_contract) = mirror_mint_contract {
        config.mirror_mint_contract = deps.api.addr_validate(&mirror_mint_contract)?;
    }
    if let Some(mirror_staking_contract) = mirror_staking_contract {
        config.mirror_staking_contract = deps.api.addr_validate(&mirror_staking_contract)?;
    }
    if let Some(mirror_gov_contract) = mirror_gov_contract {
        config.mirror_gov_contract = deps.api.addr_validate(&mirror_gov_contract)?;
    }
    if let Some(mirror_lock_contract) = mirror_lock_contract {
        config.mirror_lock_contract = deps.api.addr_validate(&mirror_lock_contract)?;
    }
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_config"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::ContractOwner {} => encode_binary(&query_contract_owner(deps)?),
        QueryMsg::Config {} => encode_binary(&query_config(deps)?),
        QueryMsg::IsOperator { address } => {
            encode_binary(&query_is_operator(deps, address.as_str())?)
        }
    }
}

pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        mirror_mint_contract: config.mirror_mint_contract.to_string(),
        mirror_staking_contract: config.mirror_staking_contract.to_string(),
        mirror_gov_contract: config.mirror_gov_contract.to_string(),
        mirror_lock_contract: config.mirror_lock_contract.to_string(),
    })
}

pub fn get_tax_deducted_funds(deps: &DepsMut, coins: Vec<Coin>) -> StdResult<Vec<Coin>> {
    if !coins.is_empty() {
        let asset = Asset {
            info: AssetInfo::NativeToken {
                denom: coins[0].denom.to_string(),
            },
            amount: coins[0].amount,
        };
        Ok(vec![asset.deduct_tax(&deps.querier)?])
    } else {
        Ok(coins)
    }
}
