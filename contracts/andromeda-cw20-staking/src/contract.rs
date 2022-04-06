use cosmwasm_bignumber::{Decimal256, Uint256};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, Addr, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Order,
    QuerierWrapper, Response, StdResult, Storage, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20ReceiveMsg;
use cw_asset::{Asset, AssetInfo, AssetInfoUnchecked};
use std::str::FromStr;

use crate::state::{
    Config, GlobalRewardInfo, Staker, State, CONFIG, GLOBAL_REWARD_INFOS, STAKERS,
    STAKER_REWARD_INFOS, STATE,
};
use ado_base::ADOContract;
use andromeda_protocol::cw20_staking::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use common::{ado_base::InstantiateMsg as BaseInstantiateMsg, error::ContractError, require};

// Version info, for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-cw20-staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let additional_reward_tokens = if let Some(additional_rewards) = msg.additional_rewards {
        let additional_rewards: StdResult<Vec<AssetInfo>> = additional_rewards
            .iter()
            .map(|r| r.check(deps.api, None))
            .collect();
        additional_rewards?
    } else {
        vec![]
    };
    for token in additional_reward_tokens.iter() {
        GLOBAL_REWARD_INFOS.save(
            deps.storage,
            &token.to_string(),
            &GlobalRewardInfo::default(),
        )?;
    }
    CONFIG.save(
        deps.storage,
        &Config {
            staking_token: msg.staking_token,
            additional_reward_tokens,
        },
    )?;
    STATE.save(
        deps.storage,
        &State {
            total_share: Uint128::zero(),
        },
    )?;

    ADOContract::default().instantiate(
        deps.storage,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "cw20_staking".to_string(),
            operators: None,
            modules: None,
            primitive_contract: None,
        },
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::AndrReceive(msg) => {
            ADOContract::default().execute(deps, env, info, msg, execute)
        }
        ExecuteMsg::AddRewardToken { asset_info } => {
            execute_add_reward_token(deps, env, info, asset_info)
        }
        ExecuteMsg::UpdateGlobalIndexes { asset_infos } => match asset_infos {
            None => execute_update_global_indexes(deps, env, info.sender.to_string(), None),
            Some(asset_infos) => {
                let asset_infos: Result<Vec<AssetInfo>, ContractError> = asset_infos
                    .iter()
                    .map(|a| Ok(a.check(deps.api, None)?))
                    .collect();
                execute_update_global_indexes(
                    deps,
                    env,
                    info.sender.to_string(),
                    Some(asset_infos?),
                )
            }
        },
        ExecuteMsg::UnstakeTokens { amount } => execute_unstake_tokens(deps, env, info, amount),
        ExecuteMsg::ClaimRewards {} => execute_claim_rewards(deps, info),
    }
}

fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&msg.msg)? {
        Cw20HookMsg::StakeTokens {} => {
            execute_stake_tokens(deps, env, msg.sender, info.sender.to_string(), msg.amount)
        }
        Cw20HookMsg::UpdateGlobalRewardIndexes {} => execute_update_global_indexes(
            deps,
            env,
            msg.sender,
            Some(vec![AssetInfo::cw20(info.sender)]),
        ),
    }
}

fn execute_add_reward_token(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_info: AssetInfoUnchecked,
) -> Result<Response, ContractError> {
    require(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    let mut config = CONFIG.load(deps.storage)?;
    let asset_info = asset_info.check(deps.api, None)?;
    require(
        !config.additional_reward_tokens.contains(&asset_info),
        ContractError::InvalidAsset {
            asset: asset_info.to_string(),
        },
    )?;

    let asset_info_string = asset_info.to_string();
    config.additional_reward_tokens.push(asset_info.clone());

    CONFIG.save(deps.storage, &config)?;

    let mut state = STATE.load(deps.storage)?;
    update_global_index(
        deps.storage,
        &deps.querier,
        env.contract.address,
        &mut state,
        &config,
        asset_info,
    )?;

    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "add_reward_token")
        .add_attribute("added_token", asset_info_string))
}

/// The foundation for this approach is inspired by Anchor's staking implementation:
/// https://github.com/Anchor-Protocol/anchor-token-contracts/blob/15c9d6f9753bd1948831f4e1b5d2389d3cf72c93/contracts/gov/src/staking.rs#L15
fn execute_stake_tokens(
    deps: DepsMut,
    env: Env,
    sender: String,
    token_address: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let config = CONFIG.load(deps.storage)?;

    let mission_contract = contract.get_mission_contract(deps.storage)?;
    let staking_token_address =
        config
            .staking_token
            .get_address(deps.api, &deps.querier, mission_contract)?;
    require(
        token_address == staking_token_address,
        ContractError::InvalidFunds {
            msg: "Deposited cw20 token is not the staking token".to_string(),
        },
    )?;

    let mut state = STATE.load(deps.storage)?;
    let mut staker = STAKERS.may_load(deps.storage, &sender)?.unwrap_or_default();

    // Update the rewards for the user. This must be done before the new share is calculated.
    update_rewards(deps.storage, &sender, &staker)?;

    let staking_token = AssetInfo::cw20(deps.api.addr_validate(&staking_token_address)?);

    // Balance already increased, so subtract deposit amount
    let total_balance = staking_token
        .query_balance(&deps.querier, env.contract.address.to_string())?
        .checked_sub(amount)?;

    let share = if total_balance.is_zero() || state.total_share.is_zero() {
        amount
    } else {
        amount.multiply_ratio(state.total_share, total_balance)
    };

    staker.share += share;
    state.total_share += share;

    STATE.save(deps.storage, &state)?;
    STAKERS.save(deps.storage, &sender, &staker)?;

    Ok(Response::new()
        .add_attribute("action", "stake_tokens")
        .add_attribute("sender", sender)
        .add_attribute("share", share)
        .add_attribute("amount", amount))
}

fn execute_unstake_tokens(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let config = CONFIG.load(deps.storage)?;
    let sender = info.sender.as_str();

    let mission_contract = contract.get_mission_contract(deps.storage)?;
    let staking_token_address =
        config
            .staking_token
            .get_address(deps.api, &deps.querier, mission_contract)?;

    let staking_token = AssetInfo::cw20(deps.api.addr_validate(&staking_token_address)?);
    let total_balance = staking_token.query_balance(&deps.querier, env.contract.address)?;

    let staker = STAKERS.may_load(deps.storage, sender)?;
    if let Some(mut staker) = staker {
        let mut state = STATE.load(deps.storage)?;
        update_rewards(deps.storage, sender, &staker)?;

        let withdraw_share = amount
            .map(|v| {
                std::cmp::max(
                    v.multiply_ratio(state.total_share, total_balance),
                    Uint128::new(1),
                )
            })
            .unwrap_or(staker.share);

        require(
            withdraw_share <= staker.share,
            ContractError::InvalidWithdrawal {
                msg: Some("Desired amount exceeds balance".to_string()),
            },
        )?;

        staker.share -= withdraw_share;
        state.total_share -= withdraw_share;

        STATE.save(deps.storage, &state)?;
        STAKERS.save(deps.storage, sender, &staker)?;

        let withdraw_amount =
            amount.unwrap_or_else(|| withdraw_share * total_balance / state.total_share);

        let asset = Asset {
            info: staking_token,
            amount: withdraw_amount,
        };
        Ok(Response::new()
            .add_attribute("action", "unstake_tokens")
            .add_attribute("sender", sender)
            .add_attribute("withdraw_amount", withdraw_amount)
            .add_attribute("withdraw_share", withdraw_share)
            .add_message(asset.transfer_msg(info.sender)?))
    } else {
        Err(ContractError::WithdrawalIsEmpty {})
    }
}

fn execute_claim_rewards(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let sender = info.sender.as_str();
    let state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    if let Some(staker) = STAKERS.may_load(deps.storage, sender)? {
        update_rewards(deps.storage, sender, &staker)?;
        let mut msgs: Vec<CosmosMsg> = vec![];

        for token in config.additional_reward_tokens {
            let token_string = token.to_string();

            // Since we call `update_rewards` first, this entry will always exist.
            let mut staker_reward_info =
                STAKER_REWARD_INFOS.load(deps.storage, (sender, &token_string))?;
            let rewards: Uint128 =
                Decimal::from(staker_reward_info.pending_rewards) * Uint128::from(1u128);

            let decimals: Decimal256 = staker_reward_info.pending_rewards
                - Decimal256::from_uint256(Uint256::from(rewards));

            if !rewards.is_zero() {
                // Reduce pending rewards for staker to what is left over after rounding.
                staker_reward_info.pending_rewards = decimals;

                let mut global_reward_info =
                    GLOBAL_REWARD_INFOS.load(deps.storage, &token_string)?;

                // Reduce reward balance.
                global_reward_info.previous_reward_balance = global_reward_info
                    .previous_reward_balance
                    .checked_sub(rewards)?;

                let asset = Asset {
                    info: AssetInfoUnchecked::from_str(&token_string)?.check(deps.api, None)?,
                    amount: rewards,
                };

                msgs.push(asset.transfer_msg(sender)?);
            }
        }

        require(!msgs.is_empty(), ContractError::WithdrawalIsEmpty {})?;

        STATE.save(deps.storage, &state)?;
        STAKERS.save(deps.storage, sender, &staker)?;
        Ok(Response::new()
            .add_attribute("action", "claim_rewards")
            .add_messages(msgs))
    } else {
        Err(ContractError::WithdrawalIsEmpty {})
    }
}

fn execute_update_global_indexes(
    deps: DepsMut,
    env: Env,
    sender: String,
    asset_infos: Option<Vec<AssetInfo>>,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    require(
        contract.is_owner_or_operator(deps.storage, &sender)?,
        ContractError::Unauthorized {},
    )?;
    let state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    let reward_tokens = asset_infos.unwrap_or(config.additional_reward_tokens.clone());

    for token in reward_tokens {
        update_global_index(
            deps.storage,
            &deps.querier,
            env.contract.address.clone(),
            &state,
            &config,
            token,
        )?;
    }
    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attribute("action", "update_global_indexes"))
}

fn update_global_index(
    storage: &mut dyn Storage,
    querier: &QuerierWrapper,
    contract_address: Addr,
    state: &State,
    config: &Config,
    token: AssetInfo,
) -> Result<(), ContractError> {
    require(
        config.additional_reward_tokens.contains(&token),
        ContractError::InvalidAsset {
            asset: token.to_string(),
        },
    )?;

    let token_string = token.to_string();
    let mut global_reward_info = GLOBAL_REWARD_INFOS
        .may_load(storage, &token_string)?
        .unwrap_or_default();

    let reward_balance = token.query_balance(querier, contract_address)?;
    let deposited_amount =
        reward_balance.checked_sub(global_reward_info.previous_reward_balance)?;

    global_reward_info.index +=
        Decimal256::from(Decimal::from_ratio(deposited_amount, state.total_share));

    global_reward_info.previous_reward_balance = reward_balance;

    GLOBAL_REWARD_INFOS.save(storage, &token_string, &global_reward_info)?;

    Ok(())
}

fn update_rewards(
    storage: &mut dyn Storage,
    staker_address: &str,
    staker: &Staker,
) -> Result<(), ContractError> {
    let reward_infos: Result<Vec<(String, GlobalRewardInfo)>, ContractError> = GLOBAL_REWARD_INFOS
        .range(storage, None, None, Order::Ascending)
        .map(|v| {
            let (token, reward_infos) = v?;
            Ok((String::from_utf8(token)?, reward_infos))
        })
        .collect();

    for (token, global_reward_info) in reward_infos? {
        let mut staker_reward_info = STAKER_REWARD_INFOS
            .may_load(storage, (staker_address, &token))?
            .unwrap_or_default();

        let staker_share = Uint256::from(staker.share);
        let rewards = (global_reward_info.index - staker_reward_info.index) * staker_share;

        staker_reward_info.index = global_reward_info.index;
        staker_reward_info.pending_rewards += Decimal256::from_uint256(rewards);

        STAKER_REWARD_INFOS.save(storage, (staker_address, &token), &staker_reward_info)?;
    }
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
    }
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
