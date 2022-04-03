#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Response,
    StdResult, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20ReceiveMsg;
use cw_asset::AssetInfo;

use crate::state::{Config, State, CONFIG, STAKERS, STATE};
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
        ExecuteMsg::WithdrawTokens { assets } => panic!(),
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

    let staking_token = AssetInfo::cw20(deps.api.addr_validate(&staking_token_address)?);
    let reward_tokens = [config.additional_reward_tokens, vec![staking_token]].concat();

    // Balance already increased, so subtract deposit amount
    let total_balance = query_token_balances(
        &deps.querier,
        reward_tokens,
        env.contract.address.to_string(),
    )?
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

fn query_token_balances(
    querier: &QuerierWrapper,
    tokens: Vec<AssetInfo>,
    address: String,
) -> Result<Uint128, ContractError> {
    let mut balance = Uint128::zero();
    for token in tokens {
        balance = balance.checked_add(token.query_balance(querier, address.clone())?)?;
    }
    Ok(balance)
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
