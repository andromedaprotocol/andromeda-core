use std::str::FromStr;

use crate::state::{DEFAULT_VALIDATOR, UNSTAKING_QUEUE};
use cosmwasm_std::{
    coin, ensure, entry_point, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut,
    DistributionMsg, Env, FullDelegation, MessageInfo, Reply, Response, StakingMsg, StdError,
    SubMsg, Timestamp, Uint128,
};
use cw2::set_contract_version;

use andromeda_finance::validator_staking::{
    is_validator, ExecuteMsg, InstantiateMsg, QueryMsg, UnstakingTokens,
};

use andromeda_std::{
    ado_base::InstantiateMsg as BaseInstantiateMsg,
    ado_contract::ADOContract,
    amp::AndrAddr,
    common::{context::ExecuteContext, encode_binary},
    error::ContractError,
};
use enum_repr::EnumRepr;

use chrono::DateTime;

const CONTRACT_NAME: &str = "crates.io:andromeda-validator-staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[EnumRepr(type = "u64")]
pub enum ReplyId {
    ValidatorUnstake = 201,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    msg.validate(&deps)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    DEFAULT_VALIDATOR.save(deps.storage, &msg.default_validator)?;

    let inst_resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        &deps.querier,
        info,
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;
    Ok(inst_resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let ctx = ExecuteContext::new(deps, info, env);

    match msg {
        ExecuteMsg::AMPReceive(pkt) => {
            ADOContract::default().execute_amp_receive(ctx, pkt, handle_execute)
        }
        _ => handle_execute(ctx, msg),
    }
}

pub fn handle_execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Stake { validator } => execute_stake(ctx, validator),
        ExecuteMsg::Unstake { validator, amount } => execute_unstake(ctx, validator, amount),
        ExecuteMsg::Claim {
            validator,
            recipient,
        } => execute_claim(ctx, validator, recipient),
        ExecuteMsg::WithdrawFunds {} => execute_withdraw_fund(ctx),

        _ => ADOContract::default().execute(ctx, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::StakedTokens { validator } => {
            encode_binary(&query_staked_tokens(deps, env.contract.address, validator)?)
        }
        QueryMsg::UnstakedTokens {} => encode_binary(&query_unstaked_tokens(deps)?),

        _ => ADOContract::default().query(deps, env, msg),
    }
}

fn execute_stake(ctx: ExecuteContext, validator: Option<Addr>) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

    // Ensure only one type of coin is received
    ensure!(
        info.funds.len() == 1,
        ContractError::ExceedsMaxAllowedCoins {}
    );

    let default_validator = DEFAULT_VALIDATOR.load(deps.storage)?;

    // Use default validator if validator is not specified by stake msg
    let validator = validator.unwrap_or(default_validator);

    // Check if the validator is valid before staking
    is_validator(&deps, &validator)?;

    // Delegate funds to the validator

    let funds = &info.funds[0];

    let res = Response::new()
        .add_message(StakingMsg::Delegate {
            validator: validator.to_string(),
            amount: funds.clone(),
        })
        .add_attribute("action", "validator-stake")
        .add_attribute("from", info.sender)
        .add_attribute("to", validator.to_string())
        .add_attribute("amount", funds.amount);

    Ok(res)
}

fn execute_unstake(
    ctx: ExecuteContext,
    validator: Option<Addr>,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;

    let delegator = env.contract.address;
    // Ensure sender is the contract owner
    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let default_validator = DEFAULT_VALIDATOR.load(deps.storage)?;
    let validator = validator.unwrap_or(default_validator);

    // Check if the validator is valid before unstaking
    is_validator(&deps, &validator)?;

    let Some(res) = deps
        .querier
        .query_delegation(delegator.to_string(), validator.to_string())?
    else {
        return Err(ContractError::InvalidValidatorOperation {
            operation: "Unstake".to_string(),
            validator: validator.to_string(),
        });
    };

    let unstake_amount = amount.unwrap_or(res.amount.amount);

    ensure!(
        !unstake_amount.is_zero() && unstake_amount <= res.amount.amount,
        ContractError::InvalidValidatorOperation {
            operation: "Unstake".to_string(),
            validator: validator.to_string(),
        }
    );

    let undelegate_msg = CosmosMsg::Staking(StakingMsg::Undelegate {
        validator: validator.to_string(),
        amount: coin(unstake_amount.u128(), res.amount.denom),
    });
    let undelegate_msg = SubMsg::reply_on_success(undelegate_msg, ReplyId::ValidatorUnstake.repr());

    let res = Response::new()
        .add_submessage(undelegate_msg)
        .add_attribute("action", "validator-unstake")
        .add_attribute("amount", unstake_amount)
        .add_attribute("from", info.sender)
        .add_attribute("to", validator.to_string());

    Ok(res)
}

fn execute_claim(
    ctx: ExecuteContext,
    validator: Option<Addr>,
    recipient: Option<AndrAddr>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;

    let default_validator = DEFAULT_VALIDATOR.load(deps.storage)?;
    let validator = validator.unwrap_or(default_validator);

    // Check if the validator is valid before unstaking
    is_validator(&deps, &validator)?;

    let recipient_address = if let Some(ref recipient) = recipient {
        recipient.get_raw_address(&deps.as_ref())?
    } else {
        info.sender
    };

    // Ensure recipient is the contract owner
    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, recipient_address.as_str())?,
        ContractError::Unauthorized {}
    );

    let delegator = env.contract.address;
    let Some(res) = deps
        .querier
        .query_delegation(delegator.to_string(), validator.to_string())?
    else {
        return Err(ContractError::InvalidValidatorOperation {
            operation: "Claim".to_string(),
            validator: validator.to_string(),
        });
    };

    // No reward to claim exist
    ensure!(
        !res.accumulated_rewards.is_empty(),
        ContractError::InvalidClaim {}
    );

    let res = Response::new()
        .add_message(DistributionMsg::SetWithdrawAddress {
            address: recipient_address.to_string(),
        })
        .add_message(DistributionMsg::WithdrawDelegatorReward {
            validator: validator.to_string(),
        })
        .add_attribute("action", "validator-claim-reward")
        .add_attribute("recipient", recipient_address)
        .add_attribute("validator", validator.to_string());

    Ok(res)
}

fn execute_withdraw_fund(ctx: ExecuteContext) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;

    // Ensure sender is the contract owner
    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let mut funds = Vec::<Coin>::new();
    loop {
        match UNSTAKING_QUEUE.front(deps.storage).unwrap() {
            Some(UnstakingTokens { payout_at, .. }) if payout_at <= env.block.time => {
                if let Some(UnstakingTokens { fund, .. }) =
                    UNSTAKING_QUEUE.pop_front(deps.storage)?
                {
                    funds.push(fund)
                }
            }
            _ => break,
        }
    }

    ensure!(
        !funds.is_empty(),
        ContractError::InvalidWithdrawal {
            msg: Some("No unstaked funds to withdraw".to_string())
        }
    );

    let res = Response::new()
        .add_message(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: funds,
        })
        .add_attribute("action", "withdraw-funds")
        .add_attribute("from", env.contract.address)
        .add_attribute("to", info.sender.into_string());

    Ok(res)
}

fn query_staked_tokens(
    deps: Deps,
    delegator: Addr,
    validator: Option<Addr>,
) -> Result<FullDelegation, ContractError> {
    let default_validator = DEFAULT_VALIDATOR.load(deps.storage)?;

    // Use default validator if validator is not specified
    let validator = validator.unwrap_or(default_validator);

    let Some(res) = deps
        .querier
        .query_delegation(delegator.to_string(), validator.to_string())?
    else {
        return Err(ContractError::InvalidDelegation {});
    };
    Ok(res)
}

fn query_unstaked_tokens(deps: Deps) -> Result<Vec<UnstakingTokens>, ContractError> {
    let iter = UNSTAKING_QUEUE.iter(deps.storage).unwrap();
    let mut res = Vec::<UnstakingTokens>::new();

    for data in iter {
        res.push(data.unwrap());
    }
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }
    match ReplyId::from_repr(msg.id) {
        Some(ReplyId::ValidatorUnstake) => on_validator_unstake(deps, msg),
        _ => Ok(Response::default()),
    }
}

pub fn on_validator_unstake(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let attributes = &msg.result.unwrap().events[0].attributes;
    let mut fund = Coin::default();
    let mut payout_at = Timestamp::default();
    for attr in attributes {
        if attr.key == "amount" {
            fund = Coin::from_str(&attr.value).unwrap();
        } else if attr.key == "completion_time" {
            let completion_time = DateTime::parse_from_rfc3339(&attr.value).unwrap();
            let seconds = completion_time.timestamp() as u64;
            let nanos = completion_time.timestamp_subsec_nanos() as u64;
            payout_at = Timestamp::from_seconds(seconds);
            payout_at = payout_at.plus_nanos(nanos);
        }
    }
    UNSTAKING_QUEUE.push_back(deps.storage, &UnstakingTokens { fund, payout_at })?;

    Ok(Response::default())
}
