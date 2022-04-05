use cosmwasm_bignumber::{Decimal256, Uint256};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, BankMsg, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    QuerierWrapper, Response, StdResult, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20ReceiveMsg;
use cw_asset::{Asset, AssetInfo, AssetInfoUnchecked, AssetUnchecked};
use std::collections::BTreeMap;
use std::str::FromStr;

use crate::state::{
    Config, ContractRewardInfo, Staker, StakerRewardInfo, State, CONFIG, STAKERS, STATE,
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
    let mut additional_reward_info: BTreeMap<String, ContractRewardInfo> = BTreeMap::new();
    for token in additional_reward_tokens.iter() {
        additional_reward_info.insert(token.to_string(), ContractRewardInfo::default());
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
            additional_reward_info,
        },
    )?;

    ADOContract::default().instantiate(
        deps.storage,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "cw20_staking".to_string(),
            operators: None,
            modules: msg.modules,
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
        ExecuteMsg::AddRewardToken { asset_info } => panic!(),
        ExecuteMsg::UpdateGlobalIndex { asset_infos } => match asset_infos {
            None => execute_update_global_index(deps, env, info.sender.to_string(), None),
            Some(asset_infos) => {
                let asset_infos: Result<Vec<AssetInfo>, ContractError> = asset_infos
                    .iter()
                    .map(|a| Ok(a.check(deps.api, None)?))
                    .collect();
                execute_update_global_index(deps, env, info.sender.to_string(), Some(asset_infos?))
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
        Cw20HookMsg::UpdateGlobalRewardIndex {} => execute_update_global_index(
            deps,
            env,
            msg.sender,
            Some(vec![AssetInfo::cw20(info.sender)]),
        ),
    }
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
    update_rewards(&state, &mut staker);

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
        update_rewards(&state, &mut staker);

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
        Ok(Response::new().add_message(asset.transfer_msg(info.sender)?))
    } else {
        Err(ContractError::WithdrawalIsEmpty {})
    }
}

fn execute_claim_rewards(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let sender = info.sender.as_str();
    let mut state = STATE.load(deps.storage)?;
    if let Some(mut staker) = STAKERS.may_load(deps.storage, sender)? {
        update_rewards(&state, &mut staker);
        let mut msgs: Vec<CosmosMsg> = vec![];

        for (token, mut staker_reward_info) in staker.reward_info.clone() {
            let rewards: Uint128 =
                Decimal::from(staker_reward_info.pending_rewards) * Uint128::from(1u128);

            let decimals: Decimal256 = staker_reward_info.pending_rewards
                - Decimal256::from_uint256(Uint256::from(rewards));

            if !rewards.is_zero() {
                // Reduce pending rewards for staker.
                staker_reward_info.pending_rewards = decimals;

                let mut contract_reward_info = state.additional_reward_info[&token].clone();

                // Reduce reward balance.
                contract_reward_info.previous_reward_balance = contract_reward_info
                    .previous_reward_balance
                    .checked_sub(rewards)?;

                state
                    .additional_reward_info
                    .insert(token.clone(), contract_reward_info);

                let asset = Asset {
                    info: AssetInfoUnchecked::from_str(&token)?.check(deps.api, None)?,
                    amount: rewards,
                };

                msgs.push(asset.transfer_msg(sender)?);

                staker.reward_info.insert(token, staker_reward_info);
            }
        }

        require(!msgs.is_empty(), ContractError::WithdrawalIsEmpty {})?;

        STATE.save(deps.storage, &state)?;
        STAKERS.save(deps.storage, sender, &staker)?;
        Ok(Response::new().add_messages(msgs))
    } else {
        Err(ContractError::WithdrawalIsEmpty {})
    }
}

fn execute_update_global_index(
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
    let mut state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    let additional_reward_tokens = config.additional_reward_tokens;
    let reward_tokens = asset_infos.unwrap_or(additional_reward_tokens.clone());

    for token in reward_tokens {
        require(
            additional_reward_tokens.contains(&token),
            ContractError::InvalidAsset {
                asset: token.to_string(),
            },
        )?;

        let token_string = token.to_string();
        if !state.additional_reward_info.contains_key(&token_string) {
            state
                .additional_reward_info
                .insert(token_string.clone(), ContractRewardInfo::default());
        }

        // Can unwrap since we it will always be there.
        let contract_reward_info = state.additional_reward_info.get_mut(&token_string).unwrap();

        let reward_balance = token.query_balance(&deps.querier, env.contract.address.clone())?;
        let deposited_amount =
            reward_balance.checked_sub(contract_reward_info.previous_reward_balance)?;

        contract_reward_info.index +=
            Decimal256::from(Decimal::from_ratio(deposited_amount, state.total_share));

        contract_reward_info.previous_reward_balance = reward_balance;
    }
    STATE.save(deps.storage, &state)?;

    Ok(Response::new())
}

fn update_rewards(state: &State, staker: &mut Staker) {
    for (token, contract_reward_info) in state.additional_reward_info.iter() {
        let token_string = token.to_string();

        if !staker.reward_info.contains_key(&token_string) {
            staker
                .reward_info
                .insert(token_string.clone(), StakerRewardInfo::default());
        }

        let staker_reward_info = staker.reward_info.get_mut(&token_string).unwrap();

        let staker_share = Uint256::from(staker.share);
        let rewards = (contract_reward_info.index - staker_reward_info.index) * staker_share;

        staker_reward_info.index = contract_reward_info.index;
        staker_reward_info.pending_rewards += Decimal256::from_uint256(rewards);
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    Ok(to_binary(&"")?)
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

#[cfg(test)]
mod tests {}
