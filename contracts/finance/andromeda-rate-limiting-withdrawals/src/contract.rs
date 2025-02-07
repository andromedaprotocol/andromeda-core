use crate::state::{ACCOUNTS, ALLOWED_COIN};
use andromeda_finance::rate_limiting_withdrawals::{
    AccountDetails, CoinAllowance, ExecuteMsg, InstantiateMsg, MinimumFrequency, QueryMsg,
};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    amp::{
        messages::{AMPCtx, AMPPkt},
        Recipient,
    },
    andr_execute_fn,
    common::{context::ExecuteContext, encode_binary, Milliseconds},
    error::ContractError,
};
use cosmwasm_std::{
    ensure, entry_point, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, SubMsg, Uint128,
};
use cw_utils::one_coin;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-rate-limiting-withdrawals";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    match msg.minimal_withdrawal_frequency {
        MinimumFrequency::Time { time } => {
            ensure!(!time.is_zero(), ContractError::InvalidZeroAmount {});

            ensure!(
                !msg.allowed_coin.limit.is_zero(),
                ContractError::InvalidZeroAmount {}
            );

            ensure!(
                !msg.allowed_coin.coin.is_empty(),
                ContractError::EmptyString {}
            );

            ALLOWED_COIN.save(
                deps.storage,
                &CoinAllowance {
                    coin: msg.allowed_coin.coin,
                    limit: msg.allowed_coin.limit,
                    minimal_withdrawal_frequency: time,
                },
            )?
        }
    }

    let inst_resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        &deps.querier,
        info.clone(),
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;

    Ok(inst_resp)
}

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit { recipient } => execute_deposit(ctx, recipient),
        ExecuteMsg::Withdraw { amount, recipient } => execute_withdraw(ctx, amount, recipient),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

fn execute_deposit(
    ctx: ExecuteContext,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;
    // The contract only supports one type of coin
    one_coin(&info)?;

    let funds = &info.funds[0];

    // Coin has to be in the allowed list
    let coin = ALLOWED_COIN.load(deps.storage)?;
    ensure!(
        coin.coin == funds.denom,
        ContractError::InvalidFunds {
            msg: "Coin must be part of the allowed list".to_string(),
        }
    );

    let user = recipient.unwrap_or(info.sender.to_string());

    // Load list of accounts
    let account = ACCOUNTS.may_load(deps.storage, user.clone())?;

    // Check if recipient already has an account
    if let Some(account) = account {
        // If the user does have an account in that coin

        // Calculate new amount of coins
        let new_amount = account.balance.checked_add(funds.amount)?;

        // add new balance with updated coin
        let new_details = AccountDetails {
            balance: new_amount,
            latest_withdrawal: account.latest_withdrawal,
        };

        // save changes
        ACCOUNTS.save(deps.storage, info.sender.to_string(), &new_details)?;

        // If user doesn't have an account at all
    } else {
        let new_account_details = AccountDetails {
            balance: funds.amount,
            latest_withdrawal: None,
        };
        // save changes
        ACCOUNTS.save(deps.storage, user, &new_account_details)?;
    }

    let res = Response::new()
        .add_attribute("action", "funded account")
        .add_attribute("account", info.sender.to_string())
        .add_attribute("amount", funds.amount);
    Ok(res)
}

fn execute_withdraw(
    ctx: ExecuteContext,
    amount: Uint128,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps,
        info,
        env,
        amp_ctx,
        contract,
        ..
    } = ctx;

    // check if sender has an account
    let account = ACCOUNTS
        .load(deps.storage, info.sender.to_string())
        .map_err(|_err| ContractError::AccountNotFound {})?;

    let allowed_coin = ALLOWED_COIN.load(deps.storage)?;

    // Calculate time since last withdrawal
    if let Some(latest_withdrawal) = account.latest_withdrawal {
        let minimum_withdrawal_frequency = allowed_coin.minimal_withdrawal_frequency;
        let current_time = Milliseconds::from_seconds(env.block.time.seconds());
        let seconds_since_withdrawal = current_time.minus_seconds(latest_withdrawal.seconds());

        // make sure enough time has elapsed since the latest withdrawal
        ensure!(
            seconds_since_withdrawal >= minimum_withdrawal_frequency,
            ContractError::FundsAreLocked {}
        );
    }

    // make sure the funds requested don't exceed the user's balance
    ensure!(
        account.balance >= amount,
        ContractError::InsufficientFunds {}
    );

    // make sure the funds don't exceed the withdrawal limit
    let limit = allowed_coin.limit;
    ensure!(limit >= amount, ContractError::WithdrawalLimitExceeded {});

    // Update amount
    let new_amount = account.balance.checked_sub(amount)?;

    // Update account details
    let new_details = AccountDetails {
        balance: new_amount,
        latest_withdrawal: Some(env.block.time),
    };

    // Save changes
    ACCOUNTS.save(deps.storage, info.sender.to_string(), &new_details)?;

    let coin = Coin {
        denom: allowed_coin.coin,
        amount,
    };

    let message: SubMsg = if let Some(recipient) = recipient {
        let amp_msg = recipient.generate_amp_msg(&deps.as_ref(), Some(vec![coin.clone()]))?;
        let ctx = if let Some(pkt) = amp_ctx {
            pkt.ctx
        } else {
            AMPCtx::new(
                info.sender.to_string(),
                env.contract.address.to_string(),
                0,
                None,
            )
        };
        let amp_pkt = AMPPkt::new_with_ctx(ctx, vec![amp_msg]);
        let kernel_address = contract.get_kernel_address(deps.storage)?;
        amp_pkt.to_sub_msg(kernel_address, Some(vec![coin.clone()]), 0)?
    } else {
        SubMsg::reply_always(
            CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![coin.clone()],
            }),
            0,
        )
    };

    // Transfer funds
    let res = Response::new()
        .add_submessage(message)
        .add_attribute("action", "withdrew funds")
        .add_attribute("coin", coin.to_string());
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::CoinAllowanceDetails {} => encode_binary(&query_coin_allowance_details(deps)?),
        QueryMsg::AccountDetails { account } => {
            encode_binary(&query_account_details(deps, account)?)
        }
        _ => ADOContract::default().query(deps, env, msg),
    }
}

fn query_account_details(deps: Deps, account: String) -> Result<AccountDetails, ContractError> {
    let user = ACCOUNTS.may_load(deps.storage, account)?;
    if let Some(details) = user {
        Ok(details)
    } else {
        Err(ContractError::AccountNotFound {})
    }
}

fn query_coin_allowance_details(deps: Deps) -> Result<CoinAllowance, ContractError> {
    let details = ALLOWED_COIN.load(deps.storage)?;
    Ok(details)
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
