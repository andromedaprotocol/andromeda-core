#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, Binary, Coin, CosmosMsg, Deps, DepsMut, DistributionMsg, Env, GovMsg, MessageInfo,
    QuerierWrapper, Response, StakingMsg, Uint128, VoteOption,
};
use cw2::{get_contract_version, set_contract_version};
use cw_asset::AssetInfo;

use cw_utils::nonpayable;
use semver::Version;
use std::cmp;

use ado_base::ADOContract;
use andromeda_finance::vesting::{
    BatchResponse, Config, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use common::{
    ado_base::InstantiateMsg as BaseInstantiateMsg,
    encode_binary,
    error::{from_semver, ContractError},
    withdraw::WithdrawalType,
};

use crate::state::{
    batches, get_all_batches_with_ids, get_claimable_batches_with_ids, save_new_batch, Batch,
    CONFIG,
};

const CONTRACT_NAME: &str = "crates.io:andromeda-vesting";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        is_multi_batch_enabled: msg.is_multi_batch_enabled,
        recipient: msg.recipient,
        denom: msg.denom,
        unbonding_duration: msg.unbonding_duration,
    };

    CONFIG.save(deps.storage, &config)?;

    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "vesting".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
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
        ExecuteMsg::AndrReceive(msg) => {
            ADOContract::default().execute(deps, env, info, msg, execute)
        }
        ExecuteMsg::CreateBatch {
            lockup_duration,
            release_unit,
            release_amount,
            validator_to_delegate_to,
        } => execute_create_batch(
            deps,
            info,
            env,
            lockup_duration,
            release_unit,
            release_amount,
            validator_to_delegate_to,
        ),
        ExecuteMsg::Claim {
            number_of_claims,
            batch_id,
        } => execute_claim(deps, env, info, number_of_claims, batch_id),
        ExecuteMsg::ClaimAll { limit, up_to_time } => {
            execute_claim_all(deps, env, info, limit, up_to_time)
        }
        ExecuteMsg::Delegate { amount, validator } => {
            execute_delegate(deps, env, info, amount, validator)
        }
        ExecuteMsg::Redelegate { amount, from, to } => {
            execute_redelegate(deps, env, info, amount, from, to)
        }
        ExecuteMsg::Undelegate { amount, validator } => {
            execute_undelegate(deps, env, info, amount, validator)
        }
        ExecuteMsg::WithdrawRewards {} => execute_withdraw_rewards(deps, env, info),
        ExecuteMsg::Vote { proposal_id, vote } => execute_vote(deps, info, proposal_id, vote),
    }
}

fn execute_create_batch(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    lockup_duration: Option<u64>,
    release_unit: u64,
    release_amount: WithdrawalType,
    validator_to_delegate_to: Option<String>,
) -> Result<Response, ContractError> {
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let config = CONFIG.load(deps.storage)?;
    let current_time = env.block.time.seconds();

    ensure!(
        info.funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Creating a batch must be accompanied with a single native fund".to_string(),
        }
    );

    let funds = info.funds[0].clone();

    ensure!(
        funds.denom == config.denom,
        ContractError::InvalidFunds {
            msg: "Invalid denom".to_string(),
        }
    );

    ensure!(
        !funds.amount.is_zero(),
        ContractError::InvalidFunds {
            msg: "Funds must be non-zero".to_string(),
        }
    );

    ensure!(
        release_unit > 0 && !release_amount.is_zero(),
        ContractError::InvalidZeroAmount {}
    );

    let lockup_end = if let Some(duration) = lockup_duration {
        current_time + duration
    } else {
        current_time
    };

    let release_amount_string = format!("{release_amount:?}");

    let batch = Batch {
        amount: funds.amount,
        amount_claimed: Uint128::zero(),
        lockup_end,
        release_unit,
        release_amount,
        last_claimed_release_time: lockup_end,
    };

    save_new_batch(deps.storage, batch, &config)?;

    let mut response = Response::new()
        .add_attribute("action", "create_batch")
        .add_attribute("amount", funds.amount)
        .add_attribute("lockup_end", lockup_end.to_string())
        .add_attribute("release_unit", release_unit.to_string())
        .add_attribute("release_amount", release_amount_string);

    if let Some(validator) = validator_to_delegate_to {
        let delegate_response = execute_delegate(deps, env, info, Some(funds.amount), validator)?;
        response = response
            .add_attributes(delegate_response.attributes)
            .add_submessages(delegate_response.messages)
            .add_events(delegate_response.events);
    }

    Ok(response)
}

fn execute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    number_of_claims: Option<u64>,
    batch_id: u64,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    // Should this be owner or recipient?
    ensure!(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let config = CONFIG.load(deps.storage)?;

    // If it doesn't exist, error will be returned to user.
    let key = batches().key(batch_id);
    let mut batch = key.load(deps.storage)?;
    let amount_to_send = claim_batch(&deps.querier, &env, &mut batch, &config, number_of_claims)?;

    ensure!(
        !amount_to_send.is_zero(),
        ContractError::WithdrawalIsEmpty {}
    );

    key.save(deps.storage, &batch)?;

    let config = CONFIG.load(deps.storage)?;
    let app_contract = contract.get_app_contract(deps.storage)?;
    let withdraw_msg = config.recipient.generate_msg_native(
        deps.api,
        &deps.querier,
        app_contract,
        vec![Coin::new(amount_to_send.u128(), config.denom)],
    )?;

    Ok(Response::new()
        .add_submessage(withdraw_msg)
        .add_attribute("action", "claim")
        .add_attribute("amount", amount_to_send)
        .add_attribute("batch_id", batch_id.to_string())
        .add_attribute("amount_left", batch.amount - batch.amount_claimed))
}

fn execute_claim_all(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    limit: Option<u32>,
    up_to_time: Option<u64>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let contract = ADOContract::default();

    ensure!(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let config = CONFIG.load(deps.storage)?;

    let current_time = env.block.time.seconds();
    let batches_with_ids = get_claimable_batches_with_ids(deps.storage, current_time, limit)?;
    let up_to_time = cmp::min(current_time, up_to_time.unwrap_or(current_time));

    let mut total_amount_to_send = Uint128::zero();
    let last_batch_id = if !batches_with_ids.is_empty() {
        batches_with_ids.last().unwrap().0.to_string()
    } else {
        "none".to_string()
    };
    for (batch_id, mut batch) in batches_with_ids {
        let key = batches().key(batch_id);

        let elapsed_time = up_to_time - batch.last_claimed_release_time;
        let num_available_claims = elapsed_time / batch.release_unit;

        let amount_to_send = claim_batch(
            &deps.querier,
            &env,
            &mut batch,
            &config,
            Some(num_available_claims),
        )?;

        total_amount_to_send += amount_to_send;

        key.save(deps.storage, &batch)?;
    }
    let mut msgs = vec![];

    // Don't want to error here since there will generally be other batches that will have
    // claimable amounts. Erroring for one would make the whole transaction fai.
    if !total_amount_to_send.is_zero() {
        let config = CONFIG.load(deps.storage)?;
        let app_contract = contract.get_app_contract(deps.storage)?;
        msgs.push(config.recipient.generate_msg_native(
            deps.api,
            &deps.querier,
            app_contract,
            vec![Coin::new(total_amount_to_send.u128(), config.denom)],
        )?)
    }
    Ok(Response::new()
        .add_submessages(msgs)
        .add_attribute("action", "claim_all")
        .add_attribute("last_batch_id_processed", last_batch_id))
}

fn execute_delegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
    validator: String,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );
    let config = CONFIG.load(deps.storage)?;
    let asset = AssetInfo::native(config.denom.clone());
    let max_amount = asset.query_balance(&deps.querier, env.contract.address)?;
    let amount = cmp::min(max_amount, amount.unwrap_or(max_amount));

    ensure!(!amount.is_zero(), ContractError::InvalidZeroAmount {});

    let msg: CosmosMsg = CosmosMsg::Staking(StakingMsg::Delegate {
        validator: validator.clone(),
        amount: Coin {
            denom: config.denom,
            amount,
        },
    });

    Ok(Response::new()
        .add_message(get_set_withdraw_address_msg(sender))
        .add_message(msg)
        .add_attribute("action", "delegate")
        .add_attribute("validator", validator)
        .add_attribute("amount", amount))
}

fn execute_redelegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
    from: String,
    to: String,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );
    let config = CONFIG.load(deps.storage)?;
    let max_amount = get_amount_delegated(
        &deps.querier,
        env.contract.address.to_string(),
        from.clone(),
    )?;
    let amount = cmp::min(max_amount, amount.unwrap_or(max_amount));

    ensure!(!amount.is_zero(), ContractError::InvalidZeroAmount {});

    let msg: CosmosMsg = CosmosMsg::Staking(StakingMsg::Redelegate {
        src_validator: from.clone(),
        dst_validator: to.clone(),
        amount: Coin {
            denom: config.denom,
            amount,
        },
    });

    Ok(Response::new()
        .add_message(get_set_withdraw_address_msg(sender))
        .add_message(msg)
        .add_attribute("action", "redelegate")
        .add_attribute("from", from)
        .add_attribute("to", to)
        .add_attribute("amount", amount))
}

fn execute_undelegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
    validator: String,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );
    let config = CONFIG.load(deps.storage)?;
    let max_amount = get_amount_delegated(
        &deps.querier,
        env.contract.address.to_string(),
        validator.clone(),
    )?;
    let amount = cmp::min(max_amount, amount.unwrap_or(max_amount));

    ensure!(!amount.is_zero(), ContractError::InvalidZeroAmount {});

    let msg: CosmosMsg = CosmosMsg::Staking(StakingMsg::Undelegate {
        validator: validator.clone(),
        amount: Coin {
            denom: config.denom,
            amount,
        },
    });

    Ok(Response::new()
        .add_message(get_set_withdraw_address_msg(sender))
        .add_message(msg)
        .add_attribute("action", "undelegate")
        .add_attribute("validator", validator)
        .add_attribute("amount", amount))
}

fn execute_withdraw_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let sender = info.sender.to_string();
    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );
    let withdraw_rewards_msgs: Vec<CosmosMsg> = deps
        .querier
        .query_all_delegations(env.contract.address)?
        .into_iter()
        .map(|d| {
            CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
                validator: d.validator,
            })
        })
        .collect();

    Ok(Response::new()
        .add_attribute("action", "withdraw_rewards")
        .add_message(get_set_withdraw_address_msg(sender))
        .add_messages(withdraw_rewards_msgs))
}

fn claim_batch(
    querier: &QuerierWrapper,
    env: &Env,
    batch: &mut Batch,
    config: &Config,
    number_of_claims: Option<u64>,
) -> Result<Uint128, ContractError> {
    let current_time = env.block.time.seconds();
    ensure!(
        batch.lockup_end <= current_time,
        ContractError::FundsAreLocked {}
    );
    let amount_per_claim = batch.release_amount.get_amount(batch.amount)?;

    let total_amount = AssetInfo::native(config.denom.to_owned())
        .query_balance(querier, env.contract.address.to_owned())?;

    let elapsed_time = current_time - batch.last_claimed_release_time;
    let num_available_claims = elapsed_time / batch.release_unit;

    let number_of_claims = cmp::min(
        number_of_claims.unwrap_or(num_available_claims),
        num_available_claims,
    );

    let amount_to_send = amount_per_claim * Uint128::from(number_of_claims);
    let amount_available = cmp::min(batch.amount - batch.amount_claimed, total_amount);

    let amount_to_send = cmp::min(amount_to_send, amount_available);

    // We dont want to update the last_claim_time when there are no funds to claim.
    if !amount_to_send.is_zero() {
        batch.amount_claimed += amount_to_send;
        batch.last_claimed_release_time += number_of_claims * batch.release_unit;
    }

    Ok(amount_to_send)
}

fn execute_vote(
    deps: DepsMut,
    info: MessageInfo,
    proposal_id: u64,
    vote: VoteOption,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    let msg: CosmosMsg = CosmosMsg::Gov(GovMsg::Vote {
        proposal_id,
        vote: vote.clone(),
    });
    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "vote")
        .add_attribute("proposal_id", proposal_id.to_string())
        .add_attribute("vote", format!("{vote:?}")))
}

fn get_amount_delegated(
    querier: &QuerierWrapper,
    delegator: String,
    validator: String,
) -> Result<Uint128, ContractError> {
    let res = querier.query_delegation(delegator, validator)?;
    match res {
        None => Ok(Uint128::zero()),
        Some(full_delegation) => Ok(full_delegation.amount.amount),
    }
}

fn get_set_withdraw_address_msg(address: String) -> CosmosMsg {
    CosmosMsg::Distribution(DistributionMsg::SetWithdrawAddress { address })
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
        }
    );

    // New version has to be newer/greater than the old version
    ensure!(
        storage_version < version,
        ContractError::CannotMigrate {
            previous_contract: stored.version,
        }
    );

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Update the ADOContract's version
    contract.execute_update_version(deps)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::Config {} => encode_binary(&query_config(deps)?),
        QueryMsg::Batch { id } => encode_binary(&query_batch(deps, env, id)?),
        QueryMsg::Batches { start_after, limit } => {
            encode_binary(&query_batches(deps, env, start_after, limit)?)
        }
    }
}

fn query_config(deps: Deps) -> Result<Config, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

fn query_batch(deps: Deps, env: Env, batch_id: u64) -> Result<BatchResponse, ContractError> {
    let batch = batches().load(deps.storage, batch_id)?;

    let config = CONFIG.load(deps.storage)?;
    get_batch_response(&deps.querier, &env, &config, batch, batch_id)
}

fn query_batches(
    deps: Deps,
    env: Env,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> Result<Vec<BatchResponse>, ContractError> {
    let batches_with_ids = get_all_batches_with_ids(deps.storage, start_after, limit)?;
    let mut batches_response = vec![];
    let config = CONFIG.load(deps.storage)?;
    for (id, batch) in batches_with_ids {
        let batch_response = get_batch_response(&deps.querier, &env, &config, batch, id)?;

        batches_response.push(batch_response);
    }
    Ok(batches_response)
}

fn get_batch_response(
    querier: &QuerierWrapper,
    env: &Env,
    config: &Config,
    mut batch: Batch,
    batch_id: u64,
) -> Result<BatchResponse, ContractError> {
    let previous_amount = batch.amount_claimed;
    let previous_last_claimed_release_time = batch.last_claimed_release_time;
    let amount_available_to_claim = if env.block.time.seconds() >= batch.lockup_end {
        claim_batch(querier, env, &mut batch, config, None)?
    } else {
        Uint128::zero()
    };
    let amount_per_release = batch.release_amount.get_amount(batch.amount)?;
    let number_of_available_claims = amount_available_to_claim / amount_per_release;
    let res = BatchResponse {
        id: batch_id,
        amount: batch.amount,
        amount_claimed: previous_amount,
        amount_available_to_claim,
        number_of_available_claims,
        lockup_end: batch.lockup_end,
        release_amount: batch.release_amount,
        release_unit: batch.release_unit,
        last_claimed_release_time: previous_last_claimed_release_time,
    };

    Ok(res)
}
