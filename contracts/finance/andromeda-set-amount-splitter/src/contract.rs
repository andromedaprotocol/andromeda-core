use std::collections::HashSet;

use crate::state::SPLITTER;
use andromeda_finance::{
    set_amount_splitter::{
        validate_recipient_list, AddressAmount, ExecuteMsg, GetSplitterConfigResponse,
        InstantiateMsg, QueryMsg, Splitter,
    },
    splitter::validate_expiry_duration,
};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    amp::messages::AMPPkt,
    common::{actions::call_action, encode_binary, expiration::Expiry, Milliseconds},
    error::ContractError,
};
use andromeda_std::{ado_contract::ADOContract, common::context::ExecuteContext};
use cosmwasm_std::{
    attr, coins, ensure, entry_point, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Reply, Response, StdError, SubMsg,
};
use cw_utils::nonpayable;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-set-amount-splitter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
// 1 day in seconds
const ONE_DAY: u64 = 86_400;
// 1 year in seconds
const ONE_YEAR: u64 = 31_536_000;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let splitter = match msg.lock_time {
        Some(ref lock_time) => {
            // New lock time can't be too short
            ensure!(
                lock_time.get_time(&env.block).seconds() >= ONE_DAY,
                ContractError::LockTimeTooShort {}
            );

            // New lock time can't be too long
            ensure!(
                lock_time.get_time(&env.block).seconds() <= ONE_YEAR,
                ContractError::LockTimeTooLong {}
            );
            Splitter {
                recipients: msg.recipients.clone(),
                lock: lock_time.get_time(&env.block),
            }
        }
        None => {
            Splitter {
                recipients: msg.recipients.clone(),
                // If locking isn't desired upon instantiation, it's automatically set to 0
                lock: Milliseconds::default(),
            }
        }
    };
    // Save kernel address after validating it

    SPLITTER.save(deps.storage, &splitter)?;

    let inst_resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        &deps.querier,
        info,
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address.clone(),
            owner: msg.owner.clone(),
        },
    )?;

    msg.validate(deps.as_ref())?;

    Ok(inst_resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    Ok(Response::default())
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

pub fn handle_execute(mut ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    let action_response = call_action(
        &mut ctx.deps,
        &ctx.info,
        &ctx.env,
        &ctx.amp_ctx,
        msg.as_ref(),
    )?;
    let res = match msg {
        ExecuteMsg::UpdateRecipients { recipients } => execute_update_recipients(ctx, recipients),
        ExecuteMsg::UpdateLock { lock_time } => execute_update_lock(ctx, lock_time),
        ExecuteMsg::Send { config } => execute_send(ctx, config),
        _ => ADOContract::default().execute(ctx, msg),
    }?;
    Ok(res
        .add_submessages(action_response.messages)
        .add_attributes(action_response.attributes)
        .add_events(action_response.events))
}

fn execute_send(
    ctx: ExecuteContext,
    config: Option<Vec<AddressAmount>>,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

    ensure!(
        info.funds.len() == 1 || info.funds.len() == 2,
        ContractError::InvalidFunds {
            msg: "A minimim of 1 and a maximum of 2 coins are allowed".to_string(),
        }
    );

    // Check against zero amounts and duplicate denoms
    let mut denom_set = HashSet::new();
    for coin in info.funds.clone() {
        ensure!(
            !coin.amount.is_zero(),
            ContractError::InvalidFunds {
                msg: "Amount must be non-zero".to_string(),
            }
        );
        ensure!(
            !denom_set.contains(&coin.denom),
            ContractError::DuplicateCoinDenoms {}
        );
        denom_set.insert(coin.denom);
    }

    let splitter = if let Some(config) = config {
        validate_recipient_list(deps.as_ref(), config.clone())?;
        config
    } else {
        SPLITTER.load(deps.storage)?.recipients
    };

    let mut msgs: Vec<SubMsg> = Vec::new();
    let mut amp_funds: Vec<Coin> = Vec::new();

    let mut pkt = AMPPkt::from_ctx(ctx.amp_ctx, ctx.env.contract.address.to_string());

    // Iterate through the sent funds
    for coin in info.funds {
        let mut remainder_funds = coin.amount;
        let denom = coin.denom;

        for recipient in splitter.clone() {
            // Find the recipient's corresponding denom for the current iteration of the sent funds
            let recipient_coin = recipient
                .coins
                .clone()
                .into_iter()
                .find(|coin| coin.denom == denom);

            if let Some(recipient_coin) = recipient_coin {
                // Deduct from total amount
                remainder_funds = remainder_funds
                    .checked_sub(recipient_coin.amount)
                    .map_err(|_| ContractError::InsufficientFunds {})?;

                let recipient_funds =
                    cosmwasm_std::coin(recipient_coin.amount.u128(), recipient_coin.denom);

                let amp_msg = recipient
                    .recipient
                    .generate_amp_msg(&deps.as_ref(), Some(vec![recipient_funds.clone()]))?;

                pkt = pkt.add_message(amp_msg);

                amp_funds.push(recipient_funds);
            }
        }

        // Refund message for sender
        if !remainder_funds.is_zero() {
            let msg = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.clone().into_string(),
                amount: coins(remainder_funds.u128(), denom),
            }));
            msgs.push(msg);
        }
    }

    let kernel_address = ADOContract::default().get_kernel_address(deps.as_ref().storage)?;

    if !pkt.messages.is_empty() {
        let distro_msg = pkt.to_sub_msg(kernel_address, Some(amp_funds), 1)?;
        msgs.push(distro_msg);
    }

    Ok(Response::new()
        .add_submessages(msgs)
        .add_attribute("action", "send")
        .add_attribute("sender", info.sender.to_string()))
}

fn execute_update_recipients(
    ctx: ExecuteContext,
    recipients: Vec<AddressAmount>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;

    nonpayable(&info)?;

    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    validate_recipient_list(deps.as_ref(), recipients.clone())?;

    let mut splitter = SPLITTER.load(deps.storage)?;
    // Can't call this function while the lock isn't expired

    ensure!(
        splitter.lock.is_expired(&env.block),
        ContractError::ContractLocked {}
    );
    // Max 100 recipients
    ensure!(
        recipients.len() <= 100,
        ContractError::ReachedRecipientLimit {}
    );

    splitter.recipients = recipients;
    SPLITTER.save(deps.storage, &splitter)?;

    Ok(Response::default().add_attributes(vec![attr("action", "update_recipients")]))
}

fn execute_update_lock(ctx: ExecuteContext, lock_time: Expiry) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;

    nonpayable(&info)?;

    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let mut splitter = SPLITTER.load(deps.storage)?;

    // Can't call this function while the lock isn't expired
    ensure!(
        splitter.lock.is_expired(&env.block),
        ContractError::ContractLocked {}
    );

    let new_expiration = validate_expiry_duration(&lock_time, &env.block)?;

    splitter.lock = new_expiration;

    SPLITTER.save(deps.storage, &splitter)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "update_lock"),
        attr("locked", new_expiration.to_string()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetSplitterConfig {} => encode_binary(&query_splitter(deps)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

fn query_splitter(deps: Deps) -> Result<GetSplitterConfigResponse, ContractError> {
    let splitter = SPLITTER.load(deps.storage)?;

    Ok(GetSplitterConfigResponse { config: splitter })
}
