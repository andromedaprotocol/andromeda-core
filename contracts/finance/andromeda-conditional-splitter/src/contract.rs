use std::{ops::Add, vec};

use crate::state::CONDITIONAL_SPLITTER;
use andromeda_finance::conditional_splitter::{
    find_threshold, validate_recipient_list, AddressFunds, ConditionalSplitter, ExecuteMsg,
    GetConditionalSplitterConfigResponse, InstantiateMsg, QueryMsg,
};

use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    amp::messages::AMPPkt,
    common::{actions::call_action, encode_binary, Milliseconds, MillisecondsDuration},
    error::ContractError,
};
use andromeda_std::{ado_contract::ADOContract, common::context::ExecuteContext};
use cosmwasm_std::{
    attr, ensure, entry_point, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Reply, Response, StdError, SubMsg, Uint128,
};
use cw_utils::nonpayable;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-conditional-splitter";
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
    let current_time = Milliseconds::from_seconds(env.block.time.seconds());

    // Construct Address Fund, funds automatically set to 0
    let mut address_funds: Vec<AddressFunds> = vec![];
    for recipient in msg.recipients {
        address_funds.push(AddressFunds::new(recipient))
    }

    let mut conditional_splitter = ConditionalSplitter {
        recipients: address_funds,
        thresholds: msg.thresholds.clone(),
        lock: msg.lock_time.unwrap_or_default(),
    };
    // Validate recipient list and thresholds
    conditional_splitter.validate(deps.as_ref())?;

    match msg.lock_time {
        Some(lock_time) => {
            // New lock time can't be too short
            ensure!(
                lock_time.seconds() >= ONE_DAY,
                ContractError::LockTimeTooShort {}
            );
            // New lock time can't be too long
            ensure!(
                lock_time.seconds() <= ONE_YEAR,
                ContractError::LockTimeTooLong {}
            );
            conditional_splitter.lock = current_time.plus_milliseconds(lock_time);
        }
        None => {
            conditional_splitter.lock = Milliseconds::default();
        }
    }
    // Save kernel address after validating it
    CONDITIONAL_SPLITTER.save(deps.storage, &conditional_splitter)?;

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
        // ExecuteMsg::UpdateRecipients { recipients } => execute_update_recipients(ctx, recipients),
        ExecuteMsg::UpdateLock { lock_time } => execute_update_lock(ctx, lock_time),
        ExecuteMsg::Send {} => execute_send(ctx),
        _ => ADOContract::default().execute(ctx, msg),
    }?;
    Ok(res
        .add_submessages(action_response.messages)
        .add_attributes(action_response.attributes)
        .add_events(action_response.events))
}

fn execute_send(ctx: ExecuteContext) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

    ensure!(
        !info.funds.is_empty(),
        ContractError::InvalidFunds {
            msg: "At least one coin should to be sent".to_string(),
        }
    );
    ensure!(
        info.funds.len() == 1,
        ContractError::ExceedsMaxAllowedCoins {}
    );
    for coin in info.funds.clone() {
        ensure!(
            !coin.amount.is_zero(),
            ContractError::InvalidFunds {
                msg: "Amount must be non-zero".to_string(),
            }
        );
    }

    let conditional_splitter = CONDITIONAL_SPLITTER.load(deps.storage)?;

    let mut msgs: Vec<SubMsg> = Vec::new();
    let mut amp_funds: Vec<Coin> = Vec::new();

    let mut remainder_funds = info.funds.clone();

    let mut pkt = AMPPkt::from_ctx(ctx.amp_ctx, ctx.env.contract.address.to_string());
    let mut recipients_with_new_funds: Vec<AddressFunds> = vec![];

    for recipient_addr in &conditional_splitter.recipients {
        // Get current range
        let threshold = find_threshold(&conditional_splitter.thresholds, recipient_addr.funds)?;
        let recipient_percent = threshold.percentage;

        let mut vec_coin: Vec<Coin> = Vec::new();
        for (i, coin) in info.funds.clone().iter().enumerate() {
            let mut recip_coin: Coin = coin.clone();

            // Difference between the range's max and current funds received
            let till_threshold = threshold.range.max - recipient_addr.funds;

            // If info.funds is greater than the below number, it means that the threshold will be surpassed.
            //TODO Multiply till_threshold with the current threshold's percentage, the additional funds (info.funds - till_threshold) will use the next threshold's percentage.
            let funds_surpass_threshold =
                till_threshold.checked_div_floor(recipient_percent).unwrap();

            // Save new amount sent
            recip_coin.amount = coin.amount * recipient_percent;

            // Save new funds
            let new_fund = recipient_addr.funds + recip_coin.amount;
            let new_address_funds = AddressFunds {
                recipient: recipient_addr.recipient.clone(),
                funds: new_fund,
            };
            recipients_with_new_funds.push(new_address_funds);

            remainder_funds[i].amount = remainder_funds[i].amount.checked_sub(recip_coin.amount)?;
            vec_coin.push(recip_coin.clone());
            amp_funds.push(recip_coin);
        }

        let amp_msg = recipient_addr
            .recipient
            .generate_amp_msg(&deps.as_ref(), Some(vec_coin))?;
        pkt = pkt.add_message(amp_msg);
    }
    let new_conditional_splitter = ConditionalSplitter {
        recipients: recipients_with_new_funds,
        thresholds: conditional_splitter.thresholds,
        lock: conditional_splitter.lock,
    };
    CONDITIONAL_SPLITTER.save(deps.storage, &new_conditional_splitter)?;

    remainder_funds.retain(|x| x.amount > Uint128::zero());

    // Why does the remaining funds go the the sender of the executor of the splitter?
    // Is it considered tax(fee) or mistake?
    // Discussion around caller of splitter function in andromedaSPLITTER smart contract.
    // From tests, it looks like owner of smart contract (Andromeda) will recieve the rest of funds.
    // If so, should be documented
    if !remainder_funds.is_empty() {
        msgs.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: remainder_funds,
        })));
    }
    let kernel_address = ADOContract::default().get_kernel_address(deps.as_ref().storage)?;
    let distro_msg = pkt.to_sub_msg(kernel_address, Some(amp_funds), 1)?;
    msgs.push(distro_msg);

    Ok(Response::new()
        .add_submessages(msgs)
        .add_attribute("action", "send")
        .add_attribute("sender", info.sender.to_string()))
}

// fn execute_update_recipients(
//     ctx: ExecuteContext,
//     recipients: Vec<AddressPercent>,
// ) -> Result<Response, ContractError> {
//     let ExecuteContext {
//         deps, info, env, ..
//     } = ctx;

//     nonpayable(&info)?;

//     ensure!(
//         ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
//         ContractError::Unauthorized {}
//     );

//     validate_recipient_list(deps.as_ref(), recipients.clone())?;

//     let mut splitter = SPLITTER.load(deps.storage)?;
//     // Can't call this function while the lock isn't expired

//     ensure!(
//         splitter.lock.is_expired(&env.block),
//         ContractError::ContractLocked {}
//     );
//     // Max 100 recipients
//     ensure!(
//         recipients.len() <= 100,
//         ContractError::ReachedRecipientLimit {}
//     );

//     splitter.recipients = recipients;
//     SPLITTER.save(deps.storage, &splitter)?;

//     Ok(Response::default().add_attributes(vec![attr("action", "update_recipients")]))
// }

fn execute_update_lock(
    ctx: ExecuteContext,
    lock_time: MillisecondsDuration,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;

    nonpayable(&info)?;

    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let mut splitter = CONDITIONAL_SPLITTER.load(deps.storage)?;

    // Can't call this function while the lock isn't expired

    ensure!(
        splitter.lock.is_expired(&env.block),
        ContractError::ContractLocked {}
    );
    // Get current time
    let current_time = Milliseconds::from_seconds(env.block.time.seconds());

    // New lock time can't be too short
    ensure!(
        lock_time.seconds() >= ONE_DAY,
        ContractError::LockTimeTooShort {}
    );

    // New lock time can't be unreasonably long
    ensure!(
        lock_time.seconds() <= ONE_YEAR,
        ContractError::LockTimeTooLong {}
    );

    // Set new lock time
    let new_expiration = current_time.plus_milliseconds(lock_time);

    splitter.lock = new_expiration;

    CONDITIONAL_SPLITTER.save(deps.storage, &splitter)?;

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

fn query_splitter(deps: Deps) -> Result<GetConditionalSplitterConfigResponse, ContractError> {
    let splitter = CONDITIONAL_SPLITTER.load(deps.storage)?;

    Ok(GetConditionalSplitterConfigResponse { config: splitter })
}
