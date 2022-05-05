#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, BlockInfo, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Storage, Timestamp, Uint128,
};

use cw0::{Duration, Expiration};
use std::cmp;

use ado_base::ADOContract;
use andromeda_finance::vesting::{ExecuteMsg, InstantiateMsg, QueryMsg};
use common::{
    ado_base::InstantiateMsg as BaseInstantiateMsg, error::ContractError, require,
    withdraw::WithdrawalType,
};

use crate::state::{get_batch_ids, save_new_batch, Batch, Config, BATCHES, CONFIG};

const CONTRACT_NAME: &str = "crates.io:andromeda-vesting";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        is_multi_batch_enabled: msg.is_multi_batch_enabled,
        recipient: msg.recipient,
        denom: msg.denom,
    };

    CONFIG.save(deps.storage, &config)?;

    ADOContract::default().instantiate(
        deps.storage,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "vesting".to_string(),
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
        ExecuteMsg::CreateBatch {
            lockup_duration,
            release_unit,
            release_amount,
            stake,
        } => execute_create_batch(
            deps,
            info,
            env,
            lockup_duration,
            release_unit,
            release_amount,
            stake,
        ),
        ExecuteMsg::Claim {
            number_of_claims,
            batch_id,
        } => execute_claim(deps, env, info, number_of_claims, batch_id),
        ExecuteMsg::ClaimAll {
            start_after,
            limit,
            up_to_time,
        } => execute_claim_all(deps, env, info, start_after, limit, up_to_time),
        ExecuteMsg::Stake { amount } => panic!(),
        ExecuteMsg::Unstake { amount } => panic!(),
    }
}

fn execute_create_batch(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    lockup_duration: Option<u64>,
    release_unit: u64,
    release_amount: WithdrawalType,
    stake: bool,
) -> Result<Response, ContractError> {
    require(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

    let config = CONFIG.load(deps.storage)?;

    require(
        info.funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Creating a batch must be accompanied with a single native fund".to_string(),
        },
    )?;

    let funds = info.funds[0];

    require(
        funds.denom == config.denom,
        ContractError::InvalidFunds {
            msg: "Invalid denom".to_string(),
        },
    )?;

    require(
        !funds.amount.is_zero(),
        ContractError::InvalidFunds {
            msg: "Funds must be non-zero".to_string(),
        },
    )?;

    require(release_unit > 0, ContractError::InvalidZeroAmount {})?;

    let (lockup_end, last_claim_time) = if let Some(duration) = lockup_duration {
        (Duration::Time(duration).after(&env.block), duration)
    } else {
        (Expiration::AtTime(env.block.time), env.block.time.seconds())
    };

    let release_amount_string = format!("{:?}", release_amount);

    let batch = Batch {
        amount: funds.amount,
        amount_claimed: Uint128::zero(),
        lockup_end,
        release_unit,
        release_amount,
        last_claim_time,
    };

    save_new_batch(deps.storage, batch)?;

    Ok(Response::new()
        .add_attribute("action", "create_batch")
        .add_attribute("amount", funds.amount)
        .add_attribute("lockup_end", lockup_end.to_string())
        .add_attribute("release_unit", release_unit.to_string())
        .add_attribute("releast_amount", release_amount_string))
}

fn execute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    number_of_claims: Option<u64>,
    batch_id: String,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    // Should this be owner or recipient?
    require(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

    // If it doesn't exist, error will be returned to user.
    let mut batch = BATCHES.load(deps.storage, &batch_id)?;

    let amount_to_send = claim_batch(deps.storage, &env.block, &mut batch, number_of_claims)?;

    BATCHES.save(deps.storage, &batch_id, &batch)?;

    let config = CONFIG.load(deps.storage)?;
    let mission_contract = contract.get_mission_contract(deps.storage)?;
    let withdraw_msg = config.recipient.generate_msg_native(
        deps.api,
        &deps.querier,
        mission_contract,
        vec![Coin::new(amount_to_send.u128(), config.denom)],
    )?;

    Ok(Response::new()
        .add_submessage(withdraw_msg)
        .add_attribute("action", "claim")
        .add_attribute("amount", amount_to_send)
        .add_attribute("batch_id", batch_id)
        .add_attribute("amount_left", batch.amount - batch.amount_claimed))
}

fn execute_claim_all(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    start_after: Option<String>,
    limit: Option<u32>,
    up_to_time: Option<u64>,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();

    require(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

    let batch_ids = get_batch_ids(deps.storage, start_after, limit)?;
    let current_time = env.block.time.seconds();
    let up_to_time = cmp::min(current_time, up_to_time.unwrap_or(current_time));

    let mut total_amount_to_send = Uint128::zero();
    let last_batch_id = if !batch_ids.is_empty() {
        batch_ids.last().unwrap()
    } else {
        "none"
    };
    for batch_id in batch_ids {
        let mut batch = BATCHES.load(deps.storage, &batch_id)?;

        let elapsed_time = up_to_time - batch.last_claim_time;
        let num_available_claims = elapsed_time / batch.release_unit;

        let amount_to_send = claim_batch(
            deps.storage,
            &env.block,
            &mut batch,
            Some(num_available_claims),
        )?;

        total_amount_to_send += amount_to_send;

        BATCHES.save(deps.storage, &batch_id, &batch)?;
    }
    let mut msgs = vec![];
    if !total_amount_to_send.is_zero() {
        let config = CONFIG.load(deps.storage)?;
        let mission_contract = contract.get_mission_contract(deps.storage)?;
        msgs.push(config.recipient.generate_msg_native(
            deps.api,
            &deps.querier,
            mission_contract,
            vec![Coin::new(total_amount_to_send.u128(), config.denom)],
        )?)
    }
    Ok(Response::new()
        .add_submessages(msgs)
        .add_attribute("action", "claim_all")
        .add_attribute("last_batch_id_processed", last_batch_id))
}

fn claim_batch(
    storage: &mut dyn Storage,
    block: &BlockInfo,
    batch: &mut Batch,
    number_of_claims: Option<u64>,
) -> Result<Uint128, ContractError> {
    require(
        batch.lockup_end.is_expired(block),
        ContractError::FundsAreLocked {},
    )?;

    let current_time = block.time.seconds();
    let elapsed_time = current_time - batch.last_claim_time;
    let num_available_claims = elapsed_time / batch.release_unit;

    let number_of_claims = cmp::min(
        number_of_claims.unwrap_or(num_available_claims),
        num_available_claims,
    );

    let amount_per_claim = batch.release_amount.get_amount(batch.amount)?;
    let amount_to_send = amount_per_claim * Uint128::from(number_of_claims);
    let amount_available = batch.amount - batch.amount_claimed;

    let amount_to_send = cmp::min(amount_to_send, amount_available);

    batch.amount_claimed += amount_to_send;
    batch.last_claim_time = current_time;

    Ok(amount_to_send)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    Ok(to_binary(&"")?)
}
