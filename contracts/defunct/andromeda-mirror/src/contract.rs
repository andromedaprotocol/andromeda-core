#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, from_binary, Addr, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    Storage, Uint128, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};

use crate::primitive_keys::{
    ADDRESSES_TO_CACHE, MIRROR_GOV, MIRROR_LOCK, MIRROR_MINT, MIRROR_MIR, MIRROR_STAKING,
};
use ado_base::state::ADOContract;
use andromeda_ecosystem::mirror::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, MirrorLockExecuteMsg,
    MirrorMintCw20HookMsg, MirrorMintExecuteMsg, MirrorStakingExecuteMsg, QueryMsg,
};
use common::{ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_asset::AssetInfo;
use terraswap::asset::AssetInfo as TerraSwapAssetInfo;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-mirror-wrapped-cdp";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let contract = ADOContract::default();
    let resp = contract.instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "mirror".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: None,
            primitive_contract: Some(msg.primitive_contract),
        },
    )?;

    for address in ADDRESSES_TO_CACHE {
        contract.cache_address(deps.storage, &deps.querier, address)?;
    }
    let mirror_token_contract = deps
        .api
        .addr_validate(&contract.get_cached_address(deps.storage, MIRROR_MIR)?)?
        .to_string();
    // We will need to be able to withdraw the MIR token.
    contract.add_withdrawable_token(
        deps.storage,
        &mirror_token_contract,
        &AssetInfo::Cw20(deps.api.addr_validate(&mirror_token_contract)?),
    )?;

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let mirror_gov_contract = contract.get_cached_address(deps.storage, MIRROR_GOV)?;
    match msg {
        ExecuteMsg::AndrReceive(msg) => {
            ADOContract::default().execute(deps, env, info, msg, execute)
        }
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
        ExecuteMsg::MirrorMintExecuteMsg(msg) => execute_mirror_mint_msg(deps, info, msg),
        ExecuteMsg::MirrorStakingExecuteMsg(msg) => execute_mirror_staking_msg(deps, info, msg),
        ExecuteMsg::MirrorGovExecuteMsg(msg) => execute_mirror_msg(
            deps,
            info.sender.to_string(),
            info.funds,
            mirror_gov_contract,
            encode_binary(&msg)?,
        ),
        ExecuteMsg::MirrorLockExecuteMsg(msg) => execute_mirror_lock_msg(deps, info, msg),
    }
}

fn execute_mirror_mint_msg(
    deps: DepsMut,
    info: MessageInfo,
    msg: MirrorMintExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let mirror_mint_contract = contract.get_cached_address(deps.storage, MIRROR_MINT)?;
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
                ts_asset_info_to_cw_asset_info(collateral.info),
                ts_asset_info_to_cw_asset_info(asset_info),
                short_params.is_some(),
            )?;

            execute_mirror_msg(
                deps,
                info.sender.to_string(),
                info.funds,
                mirror_mint_contract,
                binary,
            )
        }
        _ => execute_mirror_msg(
            deps,
            info.sender.to_string(),
            info.funds,
            mirror_mint_contract,
            binary,
        ),
    }
}

fn execute_mirror_staking_msg(
    deps: DepsMut,
    info: MessageInfo,
    msg: MirrorStakingExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let mirror_staking_contract = contract.get_cached_address(deps.storage, MIRROR_STAKING)?;
    let binary = encode_binary(&msg)?;
    match msg {
        MirrorStakingExecuteMsg::Unbond {
            asset_token,
            amount: _,
        } => {
            ADOContract::default().add_withdrawable_token(
                deps.storage,
                &asset_token,
                &AssetInfo::Cw20(deps.api.addr_validate(&asset_token)?),
            )?;

            execute_mirror_msg(
                deps,
                info.sender.to_string(),
                info.funds,
                mirror_staking_contract,
                binary,
            )
        }
        _ => execute_mirror_msg(
            deps,
            info.sender.to_string(),
            info.funds,
            mirror_staking_contract,
            binary,
        ),
    }
}

fn execute_mirror_lock_msg(
    deps: DepsMut,
    info: MessageInfo,
    msg: MirrorLockExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let mirror_lock_contract = contract.get_cached_address(deps.storage, MIRROR_LOCK)?;
    let binary = encode_binary(&msg)?;
    match msg {
        MirrorLockExecuteMsg::UnlockPositionFunds { positions_idx: _ } => {
            ADOContract::default().add_withdrawable_token(
                deps.storage,
                "uusd",
                &AssetInfo::native("uusd"),
            )?;
            execute_mirror_msg(
                deps,
                info.sender.to_string(),
                info.funds,
                mirror_lock_contract,
                binary,
            )
        }
        _ => execute_mirror_msg(
            deps,
            info.sender.to_string(),
            info.funds,
            mirror_lock_contract,
            binary,
        ),
    }
}

fn get_asset_name(asset_info: &AssetInfo) -> String {
    match asset_info {
        AssetInfo::Cw20(contract_addr) => contract_addr.to_string(),
        AssetInfo::Native(denom) => denom.clone(),
    }
}

fn handle_open_position_withdrawable_tokens(
    storage: &mut dyn Storage,
    collateral_info: AssetInfo,
    minted_asset_info: AssetInfo,
    is_short: bool,
) -> Result<(), ContractError> {
    // Barring liquidation we will want to withdraw the collateral at some point.
    ADOContract::default().add_withdrawable_token(
        storage,
        &get_asset_name(&collateral_info),
        &collateral_info,
    )?;
    if is_short {
        // If we are shorting we will get UST back eventually.
        ADOContract::default().add_withdrawable_token(
            storage,
            "uusd",
            &AssetInfo::native("uusd"),
        )?;
    } else {
        // In this case the minted assets will be immediately sent back to this contract, so
        // we want to be able to withdraw it.
        ADOContract::default().add_withdrawable_token(
            storage,
            &get_asset_name(&minted_asset_info),
            &minted_asset_info,
        )?;
    }
    Ok(())
}

pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    ensure!(
        !cw20_msg.amount.is_zero(),
        ContractError::InvalidFunds {
            msg: "Amount must be non-zero".to_string(),
        },
    )?;

    let contract = ADOContract::default();
    let mirror_staking_contract = contract.get_cached_address(deps.storage, MIRROR_STAKING)?;
    let mirror_gov_contract = contract.get_cached_address(deps.storage, MIRROR_GOV)?;

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
            mirror_staking_contract,
            encode_binary(&msg)?,
        ),
        Cw20HookMsg::MirrorGovCw20HookMsg(msg) => execute_mirror_cw20_msg(
            deps,
            cw20_msg.sender,
            token_address,
            cw20_msg.amount,
            mirror_gov_contract,
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
    let contract = ADOContract::default();
    let mirror_mint_contract = contract.get_cached_address(deps.storage, MIRROR_MINT)?;
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
                AssetInfo::Cw20(deps.api.addr_validate(&token_address)?),
                ts_asset_info_to_cw_asset_info(asset_info),
                short_params.is_some(),
            )?;
            execute_mirror_cw20_msg(
                deps,
                cw20_msg.sender,
                token_address,
                cw20_msg.amount,
                mirror_mint_contract,
                binary,
            )
        }
        _ => execute_mirror_cw20_msg(
            deps,
            cw20_msg.sender,
            token_address,
            cw20_msg.amount,
            mirror_mint_contract,
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
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, sender.as_str())?,
        ContractError::Unauthorized {}
    );
    ensure!(
        funds.is_empty() || funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Mirror expects zero or one coin to be sent".to_string(),
        },
    )?;

    let execute_msg = WasmMsg::Execute {
        contract_addr,
        funds,
        msg: msg_binary,
    };
    Ok(Response::new().add_messages(vec![CosmosMsg::Wasm(execute_msg)]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // New version
    let version: Version = CONTRACT_VERSION.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    let contract = ADOContract::default();

    ensure!(
        stored.contract == CONTRACT_NAME,
        ContractError::CannotMigrate {
            previous_contract: stored.contract,
        },
    )?;

    // New version has to be newer/greater than the old version
    ensure!(
        storage_version < version,
        ContractError::CannotMigrate {
            previous_contract: stored.version,
        },
    )?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Update the ADOContract's version
    contract.execute_update_version(deps)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
    }
}

/// Converts TerraSwapAssetInfo to cw_asset::AssetInfo. Can't use From as these are both external
/// types.
fn ts_asset_info_to_cw_asset_info(asset_info: TerraSwapAssetInfo) -> AssetInfo {
    match asset_info {
        TerraSwapAssetInfo::NativeToken { denom } => AssetInfo::Native(denom),
        TerraSwapAssetInfo::Token { contract_addr } => {
            AssetInfo::Cw20(Addr::unchecked(contract_addr))
        }
    }
}
