use crate::state::SPLITTER;
use andromeda_finance::splitter::{
    validate_recipient_list, AddressPercent, ExecuteMsg, GetSplitterConfigResponse, InstantiateMsg,
    MigrateMsg, QueryMsg, Splitter,
};

use andromeda_std::{
    ado_base::InstantiateMsg as BaseInstantiateMsg,
    amp::messages::AMPPkt,
    common::encode_binary,
    error::{from_semver, ContractError},
};
use andromeda_std::{ado_contract::ADOContract, common::context::ExecuteContext};
use cosmwasm_std::{
    attr, ensure, entry_point, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Reply, Response, StdError, SubMsg, Timestamp, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use cw_utils::{nonpayable, Expiration};
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-splitter";
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
    msg.validate()?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Max 100 recipients
    ensure!(
        msg.recipients.len() <= 100,
        ContractError::ReachedRecipientLimit {}
    );

    let current_time = env.block.time.seconds();
    let splitter = match msg.lock_time {
        Some(lock_time) => {
            // New lock time can't be too short
            ensure!(lock_time >= ONE_DAY, ContractError::LockTimeTooShort {});

            // New lock time can't be too long
            ensure!(lock_time <= ONE_YEAR, ContractError::LockTimeTooLong {});

            Splitter {
                recipients: msg.recipients,
                lock: Expiration::AtTime(Timestamp::from_seconds(lock_time + current_time)),
            }
        }
        None => {
            Splitter {
                recipients: msg.recipients,
                // If locking isn't desired upon instantiation, it's automatically set to 0
                lock: Expiration::AtTime(Timestamp::from_seconds(current_time)),
            }
        }
    };
    // Save kernel address after validating it

    SPLITTER.save(deps.storage, &splitter)?;

    let inst_resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "splitter".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            kernel_address: msg.kernel_address,
            owner: msg.owner,
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

pub fn handle_execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateRecipients { recipients } => execute_update_recipients(ctx, recipients),
        ExecuteMsg::UpdateLock { lock_time } => execute_update_lock(ctx, lock_time),
        ExecuteMsg::Send {} => execute_send(ctx),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

fn execute_send(ctx: ExecuteContext) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

    ensure!(
        !info.funds.is_empty(),
        ContractError::InvalidFunds {
            msg: "At least one coin should to be sent".to_string(),
        }
    );
    for coin in info.funds.clone() {
        ensure!(
            !coin.amount.is_zero(),
            ContractError::InvalidFunds {
                msg: "Amount must be non-zero".to_string(),
            }
        );
    }

    let splitter = SPLITTER.load(deps.storage)?;

    let mut msgs: Vec<SubMsg> = Vec::new();
    let mut amp_funds: Vec<Coin> = Vec::new();

    let mut remainder_funds = info.funds.clone();
    // Looking at this nested for loop, we could find a way to reduce time/memory complexity to avoid DoS.
    // Would like to understand more about why we loop through funds and what it exactly stored in it.
    // From there we could look into HashMaps, or other methods to break the nested loops and avoid Denial of Service.
    // [ACK-04] Limit number of coins sent to 5.
    ensure!(
        info.funds.len() < 5,
        ContractError::ExceedsMaxAllowedCoins {}
    );

    let mut pkt = AMPPkt::from_ctx(ctx.amp_ctx, ctx.env.contract.address.to_string());

    for recipient_addr in &splitter.recipients {
        let recipient_percent = recipient_addr.percent;
        let mut vec_coin: Vec<Coin> = Vec::new();
        for (i, coin) in info.funds.clone().iter().enumerate() {
            let mut recip_coin: Coin = coin.clone();
            recip_coin.amount = coin.amount * recipient_percent;
            remainder_funds[i].amount = remainder_funds[i].amount.checked_sub(recip_coin.amount)?;
            vec_coin.push(recip_coin.clone());
            amp_funds.push(recip_coin);
        }

        // let direct_message = recipient_addr
        //     .recipient
        //     .generate_direct_msg(&deps.as_ref(), vec_coin)?;
        let amp_msg = recipient_addr.recipient.generate_amp_msg(Some(vec_coin));
        pkt = pkt.add_message(amp_msg);
    }
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

fn execute_update_recipients(
    ctx: ExecuteContext,
    recipients: Vec<AddressPercent>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;

    nonpayable(&info)?;

    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    validate_recipient_list(recipients.clone())?;

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

fn execute_update_lock(ctx: ExecuteContext, lock_time: u64) -> Result<Response, ContractError> {
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
    // Get current time
    let current_time = env.block.time.seconds();

    // New lock time can't be too short
    ensure!(lock_time >= ONE_DAY, ContractError::LockTimeTooShort {});

    // New lock time can't be unreasonably long
    ensure!(lock_time <= ONE_YEAR, ContractError::LockTimeTooLong {});

    // Set new lock time
    let new_lock = Expiration::AtTime(Timestamp::from_seconds(lock_time + current_time));

    splitter.lock = new_lock;

    SPLITTER.save(deps.storage, &splitter)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "update_lock"),
        attr("locked", new_lock.to_string()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // New version
    let version: Version = CONTRACT_VERSION.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

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

    Ok(Response::default())
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
