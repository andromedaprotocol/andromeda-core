#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QueryRequest,
    Response, StdResult, Uint128, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;
use serde::de::DeserializeOwned;

use crate::state::{Config, CONFIG};
use andromeda_protocol::{
    error::ContractError,
    mirror_wrapped_cdp::{
        ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MirrorCollateralOracleQueryMsg,
        MirrorGovQueryMsg, MirrorLockQueryMsg, MirrorMintQueryMsg, MirrorOracleQueryMsg,
        MirrorStakingQueryMsg, QueryMsg,
    },
    ownership::{execute_update_owner, is_contract_owner, query_contract_owner, CONTRACT_OWNER},
    require,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use mirror_protocol::{
    collateral_oracle::{
        CollateralInfoResponse, CollateralInfosResponse, CollateralPriceResponse,
        ConfigResponse as CollateralOracleConfigResponse,
    },
    gov::{
        ConfigResponse as GovConfigResponse, PollResponse, PollsResponse, SharesResponse,
        StakerResponse, StateResponse as GovStateResponse, VotersResponse, VotersResponseItem,
    },
    lock::{ConfigResponse as LockConfigResponse, PositionLockInfoResponse},
    mint::{
        AssetConfigResponse, ConfigResponse as MintConfigResponse, NextPositionIdxResponse,
        PositionResponse, PositionsResponse,
    },
    oracle::{
        ConfigResponse as OracleConfigResponse, FeederResponse, PriceResponse, PricesResponse,
    },
    staking::{ConfigResponse as StakingConfigResponse, PoolInfoResponse, RewardInfoResponse},
};
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
    let config = Config {
        mirror_mint_contract: deps.api.addr_validate(&msg.mirror_mint_contract)?,
        mirror_staking_contract: deps.api.addr_validate(&msg.mirror_staking_contract)?,
        mirror_gov_contract: deps.api.addr_validate(&msg.mirror_gov_contract)?,
        mirror_lock_contract: deps.api.addr_validate(&msg.mirror_lock_contract)?,
        mirror_oracle_contract: deps.api.addr_validate(&msg.mirror_oracle_contract)?,
        mirror_collateral_oracle_contract: deps
            .api
            .addr_validate(&msg.mirror_collateral_oracle_contract)?,
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
            info,
            config.mirror_mint_contract.to_string(),
            to_binary(&msg)?,
        ),
        ExecuteMsg::MirrorStakingExecuteMsg(msg) => execute_mirror_msg(
            deps,
            info,
            config.mirror_staking_contract.to_string(),
            to_binary(&msg)?,
        ),
        ExecuteMsg::MirrorGovExecuteMsg(msg) => execute_mirror_msg(
            deps,
            info,
            config.mirror_gov_contract.to_string(),
            to_binary(&msg)?,
        ),
        ExecuteMsg::MirrorLockExecuteMsg(msg) => execute_mirror_msg(
            deps,
            info,
            config.mirror_lock_contract.to_string(),
            to_binary(&msg)?,
        ),
        ExecuteMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
        ExecuteMsg::UpdateConfig {
            mirror_mint_contract,
            mirror_staking_contract,
            mirror_gov_contract,
            mirror_lock_contract,
            mirror_oracle_contract,
            mirror_collateral_oracle_contract,
        } => execute_update_config(
            deps,
            info,
            mirror_mint_contract,
            mirror_staking_contract,
            mirror_gov_contract,
            mirror_lock_contract,
            mirror_oracle_contract,
            mirror_collateral_oracle_contract,
        ),
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
            info,
            token_address,
            cw20_msg.amount,
            config.mirror_mint_contract.to_string(),
            to_binary(&msg)?,
        ),
        Cw20HookMsg::MirrorStakingCw20HookMsg(msg) => execute_mirror_cw20_msg(
            deps,
            info,
            token_address,
            cw20_msg.amount,
            config.mirror_staking_contract.to_string(),
            to_binary(&msg)?,
        ),
        Cw20HookMsg::MirrorGovCw20HookMsg(msg) => execute_mirror_cw20_msg(
            deps,
            info,
            token_address,
            cw20_msg.amount,
            config.mirror_gov_contract.to_string(),
            to_binary(&msg)?,
        ),
    }
}

pub fn execute_mirror_cw20_msg(
    deps: DepsMut,
    info: MessageInfo,
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
    execute_mirror_msg(deps, info, token_addr, to_binary(&msg)?)
}

pub fn execute_mirror_msg(
    deps: DepsMut,
    info: MessageInfo,
    contract_addr: String,
    msg_binary: Binary,
) -> Result<Response, ContractError> {
    require(
        info.funds.is_empty() || info.funds.len() == 1,
        ContractError::InvalidMirrorFunds {
            msg: "Mirror expects no funds or a single type of fund to be deposited.".to_string(),
        },
    )?;
    let tax_deducted_funds = get_tax_deducted_funds(&deps, info.funds)?;

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
    mirror_oracle_contract: Option<String>,
    mirror_collateral_oracle_contract: Option<String>,
) -> Result<Response, ContractError> {
    require(
        is_contract_owner(deps.storage, info.sender.to_string())?,
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
    if let Some(mirror_oracle_contract) = mirror_oracle_contract {
        config.mirror_oracle_contract = deps.api.addr_validate(&mirror_oracle_contract)?;
    }
    if let Some(mirror_collateral_oracle_contract) = mirror_collateral_oracle_contract {
        config.mirror_collateral_oracle_contract =
            deps.api.addr_validate(&mirror_collateral_oracle_contract)?;
    }
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ContractOwner {} => to_binary(&query_contract_owner(deps)?),
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::MirrorMintQueryMsg(msg) => query_mirror_mint(deps, msg),
        QueryMsg::MirrorStakingQueryMsg(msg) => query_mirror_staking(deps, msg),
        QueryMsg::MirrorGovQueryMsg(msg) => query_mirror_gov(deps, msg),
        QueryMsg::MirrorLockQueryMsg(msg) => query_mirror_lock(deps, msg),
        QueryMsg::MirrorOracleQueryMsg(msg) => query_mirror_oracle(deps, msg),
        QueryMsg::MirrorCollateralOracleQueryMsg(msg) => query_mirror_collateral_oracle(deps, msg),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        mirror_mint_contract: config.mirror_mint_contract.to_string(),
        mirror_staking_contract: config.mirror_staking_contract.to_string(),
        mirror_gov_contract: config.mirror_gov_contract.to_string(),
        mirror_lock_contract: config.mirror_lock_contract.to_string(),
        mirror_oracle_contract: config.mirror_oracle_contract.to_string(),
        mirror_collateral_oracle_contract: config.mirror_collateral_oracle_contract.to_string(),
    })
}

pub fn query_mirror_mint(deps: Deps, msg: MirrorMintQueryMsg) -> StdResult<Binary> {
    let contract_addr = CONFIG.load(deps.storage)?.mirror_mint_contract.to_string();
    match msg {
        MirrorMintQueryMsg::Config {} => to_binary(&query_mirror_msg::<MintConfigResponse>(
            deps,
            contract_addr,
            to_binary(&msg)?,
        )?),
        MirrorMintQueryMsg::AssetConfig { .. } => {
            to_binary(&query_mirror_msg::<AssetConfigResponse>(
                deps,
                contract_addr,
                to_binary(&msg)?,
            )?)
        }
        MirrorMintQueryMsg::Position { .. } => to_binary(&query_mirror_msg::<PositionResponse>(
            deps,
            contract_addr,
            to_binary(&msg)?,
        )?),
        MirrorMintQueryMsg::Positions { .. } => to_binary(&query_mirror_msg::<PositionsResponse>(
            deps,
            contract_addr,
            to_binary(&msg)?,
        )?),
        MirrorMintQueryMsg::NextPositionIdx {} => {
            to_binary(&query_mirror_msg::<NextPositionIdxResponse>(
                deps,
                contract_addr,
                to_binary(&msg)?,
            )?)
        }
    }
}

pub fn query_mirror_staking(deps: Deps, msg: MirrorStakingQueryMsg) -> StdResult<Binary> {
    let contract_addr = CONFIG
        .load(deps.storage)?
        .mirror_staking_contract
        .to_string();
    match msg {
        MirrorStakingQueryMsg::Config {} => to_binary(&query_mirror_msg::<StakingConfigResponse>(
            deps,
            contract_addr,
            to_binary(&msg)?,
        )?),
        MirrorStakingQueryMsg::PoolInfo { .. } => to_binary(&query_mirror_msg::<PoolInfoResponse>(
            deps,
            contract_addr,
            to_binary(&msg)?,
        )?),
        MirrorStakingQueryMsg::RewardInfo { .. } => {
            to_binary(&query_mirror_msg::<RewardInfoResponse>(
                deps,
                contract_addr,
                to_binary(&msg)?,
            )?)
        }
    }
}

pub fn query_mirror_gov(deps: Deps, msg: MirrorGovQueryMsg) -> StdResult<Binary> {
    let contract_addr = CONFIG.load(deps.storage)?.mirror_gov_contract.to_string();
    match msg {
        MirrorGovQueryMsg::Config {} => to_binary(&query_mirror_msg::<GovConfigResponse>(
            deps,
            contract_addr,
            to_binary(&msg)?,
        )?),
        MirrorGovQueryMsg::State {} => to_binary(&query_mirror_msg::<GovStateResponse>(
            deps,
            contract_addr,
            to_binary(&msg)?,
        )?),
        MirrorGovQueryMsg::Staker { .. } => to_binary(&query_mirror_msg::<StakerResponse>(
            deps,
            contract_addr,
            to_binary(&msg)?,
        )?),
        MirrorGovQueryMsg::Poll { .. } => to_binary(&query_mirror_msg::<PollResponse>(
            deps,
            contract_addr,
            to_binary(&msg)?,
        )?),
        MirrorGovQueryMsg::Polls { .. } => to_binary(&query_mirror_msg::<PollsResponse>(
            deps,
            contract_addr,
            to_binary(&msg)?,
        )?),
        MirrorGovQueryMsg::Voter { .. } => to_binary(&query_mirror_msg::<VotersResponseItem>(
            deps,
            contract_addr,
            to_binary(&msg)?,
        )?),
        MirrorGovQueryMsg::Voters { .. } => to_binary(&query_mirror_msg::<VotersResponse>(
            deps,
            contract_addr,
            to_binary(&msg)?,
        )?),
        MirrorGovQueryMsg::Shares { .. } => to_binary(&query_mirror_msg::<SharesResponse>(
            deps,
            contract_addr,
            to_binary(&msg)?,
        )?),
    }
}

pub fn query_mirror_lock(deps: Deps, msg: MirrorLockQueryMsg) -> StdResult<Binary> {
    let contract_addr = CONFIG.load(deps.storage)?.mirror_lock_contract.to_string();
    match msg {
        MirrorLockQueryMsg::Config {} => to_binary(&query_mirror_msg::<LockConfigResponse>(
            deps,
            contract_addr,
            to_binary(&msg)?,
        )?),
        MirrorLockQueryMsg::PositionLockInfo { .. } => {
            to_binary(&query_mirror_msg::<PositionLockInfoResponse>(
                deps,
                contract_addr,
                to_binary(&msg)?,
            )?)
        }
    }
}

pub fn query_mirror_oracle(deps: Deps, msg: MirrorOracleQueryMsg) -> StdResult<Binary> {
    let contract_addr = CONFIG
        .load(deps.storage)?
        .mirror_oracle_contract
        .to_string();
    match msg {
        MirrorOracleQueryMsg::Config {} => to_binary(&query_mirror_msg::<OracleConfigResponse>(
            deps,
            contract_addr,
            to_binary(&msg)?,
        )?),
        MirrorOracleQueryMsg::Feeder { .. } => to_binary(&query_mirror_msg::<FeederResponse>(
            deps,
            contract_addr,
            to_binary(&msg)?,
        )?),
        MirrorOracleQueryMsg::Price { .. } => to_binary(&query_mirror_msg::<PriceResponse>(
            deps,
            contract_addr,
            to_binary(&msg)?,
        )?),
        MirrorOracleQueryMsg::Prices { .. } => to_binary(&query_mirror_msg::<PricesResponse>(
            deps,
            contract_addr,
            to_binary(&msg)?,
        )?),
    }
}

pub fn query_mirror_collateral_oracle(
    deps: Deps,
    msg: MirrorCollateralOracleQueryMsg,
) -> StdResult<Binary> {
    let contract_addr = CONFIG
        .load(deps.storage)?
        .mirror_collateral_oracle_contract
        .to_string();
    match msg {
        MirrorCollateralOracleQueryMsg::Config {} => {
            to_binary(&query_mirror_msg::<CollateralOracleConfigResponse>(
                deps,
                contract_addr,
                to_binary(&msg)?,
            )?)
        }
        MirrorCollateralOracleQueryMsg::CollateralPrice { .. } => {
            to_binary(&query_mirror_msg::<CollateralPriceResponse>(
                deps,
                contract_addr,
                to_binary(&msg)?,
            )?)
        }
        MirrorCollateralOracleQueryMsg::CollateralAssetInfo { .. } => to_binary(
            &query_mirror_msg::<CollateralInfoResponse>(deps, contract_addr, to_binary(&msg)?)?,
        ),
        MirrorCollateralOracleQueryMsg::CollateralAssetInfos { .. } => to_binary(
            &query_mirror_msg::<CollateralInfosResponse>(deps, contract_addr, to_binary(&msg)?)?,
        ),
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
