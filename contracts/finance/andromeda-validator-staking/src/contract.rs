use crate::state::{DEFAULT_VALIDATOR, UNSTAKING_QUEUE};
use cosmwasm_std::{
    ensure, entry_point, Addr, BankMsg, Binary, Coin, Deps, DepsMut, DistributionMsg, Env,
    FullDelegation, MessageInfo, Response, StakingMsg,
};
use cw2::set_contract_version;

use andromeda_finance::validator_staking::{
    is_validator, ExecuteMsg, InstantiateMsg, QueryMsg, Unstaking,
};

use andromeda_std::{
    ado_base::InstantiateMsg as BaseInstantiateMsg,
    ado_contract::ADOContract,
    amp::AndrAddr,
    common::{context::ExecuteContext, encode_binary},
    error::ContractError,
};

const CONTRACT_NAME: &str = "crates.io:andromeda-validator-staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
        info,
        BaseInstantiateMsg {
            ado_type: "validator-staking".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
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
        ExecuteMsg::Unstake { validator } => execute_unstake(ctx, validator),
        ExecuteMsg::Claim {
            validator,
            recipient,
        } => execute_claim(ctx, validator, recipient),
        ExecuteMsg::WithdrawFund {} => execute_withdraw_fund(ctx),

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

    let Some(res) = deps.querier.query_delegation(delegator.to_string(), validator.to_string())? else {
        return Err(ContractError::InvalidValidatorOperation { operation: "Unstake".to_string(), validator: validator.to_string() });
    };

    ensure!(
        !res.amount.amount.is_zero(),
        ContractError::InvalidValidatorOperation {
            operation: "Unstake".to_string(),
            validator: validator.to_string(),
        }
    );

    UNSTAKING_QUEUE.push_back(
        deps.storage,
        &Unstaking {
            fund: res.amount.clone(),
            payout_at: env.block.time.plus_days(21),
        },
    )?;

    let res = Response::new()
        .add_message(StakingMsg::Undelegate {
            validator: validator.to_string(),
            amount: res.amount,
        })
        .add_attribute("action", "validator-unstake")
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

    // Ensure sender is the contract owner
    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let default_validator = DEFAULT_VALIDATOR.load(deps.storage)?;
    let validator = validator.unwrap_or(default_validator);

    // Check if the validator is valid before unstaking
    is_validator(&deps, &validator)?;

    let recipient = if let Some(recipient) = recipient {
        recipient.get_raw_address(&deps.as_ref())?
    } else {
        info.sender
    };

    // Ensure recipient is the contract owner
    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, recipient.as_str())?,
        ContractError::Unauthorized {}
    );

    let delegator = env.contract.address;
    let Some(res) = deps.querier.query_delegation(delegator.to_string(), validator.to_string())? else {
        return Err(ContractError::InvalidValidatorOperation { operation: "Claim".to_string(), validator: validator.to_string() });
    };

    // No reward to claim exist
    ensure!(
        !res.accumulated_rewards.is_empty(),
        ContractError::InvalidClaim {}
    );

    let res = Response::new()
        .add_message(DistributionMsg::SetWithdrawAddress {
            address: recipient.to_string(),
        })
        .add_message(DistributionMsg::WithdrawDelegatorReward {
            validator: validator.to_string(),
        })
        .add_attribute("action", "validator-claim-reward")
        .add_attribute("recipient", recipient)
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

    // let res = Response::new();
    let mut funds = Vec::<Coin>::new();
    loop {
        match UNSTAKING_QUEUE.front(deps.storage).unwrap() {
            Some(Unstaking { payout_at, .. }) if payout_at <= env.block.time => {
                if let Some(Unstaking { fund, .. }) = UNSTAKING_QUEUE.pop_front(deps.storage)? {
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

    let Some(res) = deps.querier.query_delegation(delegator.to_string(), validator.to_string())? else {
        return Err(ContractError::InvalidDelegation {});
    };
    Ok(res)
}

fn query_unstaked_tokens(deps: Deps) -> Result<Vec<Unstaking>, ContractError> {
    let iter = UNSTAKING_QUEUE.iter(deps.storage).unwrap();
    let mut res = Vec::<Unstaking>::new();

    for data in iter {
        res.push(data.unwrap());
    }
    Ok(res)
}
