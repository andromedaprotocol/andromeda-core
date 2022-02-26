#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, Storage,
    Uint128, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};

use crate::state::{Config, CONFIG};
use andromeda_protocol::{
    common::get_tax_deducted_funds,
    communication::{encode_binary, parse_message, AndromedaMsg, AndromedaQuery},
    error::ContractError,
    mirror_wrapped_cdp::{
        ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, MirrorLockExecuteMsg,
        MirrorMintCw20HookMsg, MirrorMintExecuteMsg, MirrorStakingExecuteMsg, QueryMsg,
    },
    operators::{
        execute_update_operators, initialize_operators, is_operator, query_is_operator,
        query_operators,
    },
    ownership::{execute_update_owner, is_contract_owner, query_contract_owner, CONTRACT_OWNER},
    require,
    swapper::AssetInfo,
    withdraw::{add_withdrawable_token, execute_withdraw},
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

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
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    if let Some(operators) = msg.operators {
        initialize_operators(deps.storage, operators)?;
    }
    let config = Config {
        mirror_mint_contract: deps.api.addr_validate(&msg.mirror_mint_contract)?,
        mirror_staking_contract: deps.api.addr_validate(&msg.mirror_staking_contract)?,
        mirror_gov_contract: deps.api.addr_validate(&msg.mirror_gov_contract)?,
        mirror_lock_contract: deps.api.addr_validate(&msg.mirror_lock_contract)?,
    };
    let mirror_token_contract = deps
        .api
        .addr_validate(&msg.mirror_token_contract)?
        .to_string();
    // We will need to be able to withdraw the MIR token.
    add_withdrawable_token(
        deps.storage,
        &mirror_token_contract,
        &AssetInfo::Token {
            contract_addr: deps.api.addr_validate(&mirror_token_contract)?,
        },
    )?;
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
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    match msg {
        ExecuteMsg::AndrReceive(msg) => execute_andr_receive(deps, env, info, msg),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
        ExecuteMsg::MirrorMintExecuteMsg(msg) => execute_mirror_mint_msg(deps, info, msg),
        ExecuteMsg::MirrorStakingExecuteMsg(msg) => execute_mirror_staking_msg(deps, info, msg),
        ExecuteMsg::MirrorGovExecuteMsg(msg) => execute_mirror_msg(
            deps,
            info.sender.to_string(),
            info.funds,
            config.mirror_gov_contract.to_string(),
            encode_binary(&msg)?,
        ),
        ExecuteMsg::MirrorLockExecuteMsg(msg) => execute_mirror_lock_msg(deps, info, msg),
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
    }
}

fn execute_mirror_mint_msg(
    deps: DepsMut,
    info: MessageInfo,
    msg: MirrorMintExecuteMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let binary = encode_binary(&msg)?;
    match msg {
        MirrorMintExecuteMsg::OpenPosition {
            collateral,
            asset_info,
            collateral_ratio: _,
            short_params,
        } => {
            handle_open_position_withdrawable_tokens(
                deps.storage,
                collateral.info.into(),
                asset_info.into(),
                short_params.is_some(),
            )?;

            execute_mirror_msg(
                deps,
                info.sender.to_string(),
                info.funds,
                config.mirror_mint_contract.to_string(),
                binary,
            )
        }
        _ => execute_mirror_msg(
            deps,
            info.sender.to_string(),
            info.funds,
            config.mirror_mint_contract.to_string(),
            binary,
        ),
    }
}

fn execute_mirror_staking_msg(
    deps: DepsMut,
    info: MessageInfo,
    msg: MirrorStakingExecuteMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let binary = encode_binary(&msg)?;
    match msg {
        MirrorStakingExecuteMsg::Unbond {
            asset_token,
            amount: _,
        } => {
            add_withdrawable_token(
                deps.storage,
                &asset_token,
                &AssetInfo::Token {
                    contract_addr: deps.api.addr_validate(&asset_token)?,
                },
            )?;

            execute_mirror_msg(
                deps,
                info.sender.to_string(),
                info.funds,
                config.mirror_staking_contract.to_string(),
                binary,
            )
        }
        _ => execute_mirror_msg(
            deps,
            info.sender.to_string(),
            info.funds,
            config.mirror_staking_contract.to_string(),
            binary,
        ),
    }
}

fn execute_mirror_lock_msg(
    deps: DepsMut,
    info: MessageInfo,
    msg: MirrorLockExecuteMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let binary = encode_binary(&msg)?;
    match msg {
        MirrorLockExecuteMsg::UnlockPositionFunds { positions_idx: _ } => {
            add_withdrawable_token(
                deps.storage,
                "uusd",
                &AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
            )?;
            execute_mirror_msg(
                deps,
                info.sender.to_string(),
                info.funds,
                config.mirror_lock_contract.to_string(),
                binary,
            )
        }
        _ => execute_mirror_msg(
            deps,
            info.sender.to_string(),
            info.funds,
            config.mirror_lock_contract.to_string(),
            binary,
        ),
    }
}

fn get_asset_name(asset_info: &AssetInfo) -> String {
    match asset_info {
        AssetInfo::Token { contract_addr } => contract_addr.to_string(),
        AssetInfo::NativeToken { denom } => denom.clone(),
    }
}

fn handle_open_position_withdrawable_tokens(
    storage: &mut dyn Storage,
    collateral_info: AssetInfo,
    minted_asset_info: AssetInfo,
    is_short: bool,
) -> Result<(), ContractError> {
    // Barring liquidation we will want to withdraw the collateral at some point.
    add_withdrawable_token(storage, &get_asset_name(&collateral_info), &collateral_info)?;
    if is_short {
        // If we are shorting we will get UST back eventually.
        add_withdrawable_token(
            storage,
            "uusd",
            &AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        )?;
    } else {
        // In this case the minted assets will be immediately sent back to this contract, so
        // we want to be able to withdraw it.
        add_withdrawable_token(
            storage,
            &get_asset_name(&minted_asset_info),
            &minted_asset_info,
        )?;
    }
    Ok(())
}

fn execute_andr_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AndromedaMsg,
) -> Result<Response, ContractError> {
    match msg {
        AndromedaMsg::Receive(data) => {
            let received: ExecuteMsg = parse_message(data)?;
            match received {
                ExecuteMsg::AndrReceive(..) => Err(ContractError::NestedAndromedaMsg {}),
                _ => execute(deps, env, info, received),
            }
        }
        AndromedaMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
        AndromedaMsg::UpdateOperators { operators } => {
            execute_update_operators(deps, info, operators)
        }
        AndromedaMsg::Withdraw {
            recipient,
            tokens_to_withdraw,
        } => execute_withdraw(deps.as_ref(), env, info, recipient, tokens_to_withdraw),
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
        Cw20HookMsg::MirrorMintCw20HookMsg(msg) => {
            execute_mirror_mint_cw20_msg(deps, info, cw20_msg, msg)
        }
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

fn execute_mirror_mint_cw20_msg(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
    mirror_msg: MirrorMintCw20HookMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let token_address = info.sender.to_string();
    let binary = encode_binary(&mirror_msg)?;
    match mirror_msg {
        MirrorMintCw20HookMsg::OpenPosition {
            asset_info,
            collateral_ratio: _,
            short_params,
        } => {
            handle_open_position_withdrawable_tokens(
                deps.storage,
                AssetInfo::Token {
                    contract_addr: deps.api.addr_validate(&token_address)?,
                },
                asset_info.into(),
                short_params.is_some(),
            )?;
            execute_mirror_cw20_msg(
                deps,
                cw20_msg.sender,
                token_address,
                cw20_msg.amount,
                config.mirror_mint_contract.to_string(),
                binary,
            )
        }
        _ => execute_mirror_cw20_msg(
            deps,
            cw20_msg.sender,
            token_address,
            cw20_msg.amount,
            config.mirror_mint_contract.to_string(),
            binary,
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
        ContractError::InvalidFunds {
            msg: "Mirror expects zero or one coin to be sent".to_string(),
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
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => handle_andromeda_query(deps, env, msg),
        QueryMsg::Config {} => encode_binary(&query_config(deps)?),
    }
}

fn handle_andromeda_query(
    deps: Deps,
    env: Env,
    msg: AndromedaQuery,
) -> Result<Binary, ContractError> {
    match msg {
        AndromedaQuery::Get(data) => {
            let received: QueryMsg = parse_message(data)?;
            match received {
                QueryMsg::AndrQuery(..) => Err(ContractError::NestedAndromedaMsg {}),
                _ => query(deps, env, received),
            }
        }
        AndromedaQuery::Owner {} => encode_binary(&query_contract_owner(deps)?),
        AndromedaQuery::Operators {} => encode_binary(&query_operators(deps)?),
        AndromedaQuery::IsOperator { address } => {
            encode_binary(&query_is_operator(deps, &address)?)
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
