use crate::state::{ACCOUNTS, ALLOWED_COIN};

use andromeda_finance::rate_limiting_withdrawals::{
    AccountDetails, CoinAllowance, ExecuteMsg, InstantiateMsg, MigrateMsg, MinimumFrequency,
    QueryMsg,
};
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::common::context::ExecuteContext;
use andromeda_std::{
    ado_base::{hooks::AndromedaHook, InstantiateMsg as BaseInstantiateMsg},
    common::encode_binary,
    error::{from_semver, ContractError},
};

use cosmwasm_std::{
    ensure, entry_point, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Response, Uint128,
};
use cw2::{get_contract_version, set_contract_version};

use cw_utils::{nonpayable, one_coin};
use semver::Version;

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
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    match msg.minimal_withdrawal_frequency {
        MinimumFrequency::Time { time } => ALLOWED_COIN.save(
            deps.storage,
            &CoinAllowance {
                coin: msg.allowed_coin.coin,
                limit: msg.allowed_coin.limit,
                minimal_withdrawal_frequency: time,
            },
        )?,
        //NOTE temporary until a replacement for primitive is implemented
        _ => ALLOWED_COIN.save(
            deps.storage,
            &CoinAllowance {
                coin: msg.allowed_coin.coin,
                limit: msg.allowed_coin.limit,
                minimal_withdrawal_frequency: Uint128::zero(),
            },
        )?,
        // MinimumFrequency::AddressAndKey { address_and_key } => ALLOWED_COIN.save(
        //     deps.storage,
        //     &CoinAllowance {
        //         coin: msg.allowed_coin.clone().coin,
        //         limit: msg.allowed_coin.limit,
        //         minimal_withdrawal_frequency: query_primitive::<GetValueResponse>(
        //             deps.querier,
        //             address_and_key.contract_address,
        //             address_and_key.key,
        //         )?
        //         .value
        //         .try_get_uint128()?,
        //     },
        // )?,
    }

    let inst_resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info.clone(),
        BaseInstantiateMsg {
            ado_type: "rate-limiting-withdrawals".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;
    let mod_resp =
        ADOContract::default().register_modules(info.sender.as_str(), deps.storage, msg.modules)?;

    Ok(inst_resp
        .add_attributes(mod_resp.attributes)
        .add_submessages(mod_resp.messages))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();

    if !matches!(msg, ExecuteMsg::UpdateAppContract { .. })
        && !matches!(msg, ExecuteMsg::UpdateOwner { .. })
    {
        contract.module_hook::<Response>(
            &deps.as_ref(),
            AndromedaHook::OnExecute {
                sender: info.sender.to_string(),
                payload: encode_binary(&msg)?,
            },
        )?;
    }

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
        ExecuteMsg::Deposits { recipient } => execute_deposit(ctx, recipient),
        ExecuteMsg::Withdraws { amount } => execute_withdraw(ctx, amount),
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

    // Coin has to be in the allowed list
    let coin = ALLOWED_COIN.load(deps.storage)?;
    ensure!(
        coin.coin == info.funds[0].denom,
        ContractError::InvalidFunds {
            msg: "Coin must be part of the allowed list".to_string(),
        }
    );

    let user = recipient.unwrap_or_else(|| info.sender.to_string());

    // Load list of accounts
    let account = ACCOUNTS.may_load(deps.storage, user.clone())?;

    // Check if recipient already has an account
    if let Some(account) = account {
        // If the user does have an account in that coin

        // Calculate new amount of coins
        let new_amount = account.balance + info.funds[0].amount;

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
            balance: info.funds[0].amount,
            latest_withdrawal: None,
        };
        // save changes
        ACCOUNTS.save(deps.storage, user, &new_account_details)?;
    }

    let res = Response::new()
        .add_attribute("action", "funded account")
        .add_attribute("account", info.sender.to_string());
    Ok(res)
}

fn execute_withdraw(ctx: ExecuteContext, amount: Uint128) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;

    nonpayable(&info)?;
    // check if sender has an account
    let account = ACCOUNTS.may_load(deps.storage, info.sender.to_string())?;
    if let Some(account) = account {
        // Calculate time since last withdrawal
        if let Some(latest_withdrawal) = account.latest_withdrawal {
            let minimum_withdrawal_frequency = ALLOWED_COIN
                .load(deps.storage)?
                .minimal_withdrawal_frequency;
            let current_time = Uint128::from(env.block.time.seconds());
            let seconds_since_withdrawal =
                current_time - Uint128::from(latest_withdrawal.seconds());

            // make sure enough time has elapsed since the latest withdrawal
            ensure!(
                seconds_since_withdrawal >= minimum_withdrawal_frequency,
                ContractError::FundsAreLocked {}
            );

            // make sure the funds requested don't exceed the user's balance
            ensure!(
                account.balance >= amount,
                ContractError::InsufficientFunds {}
            );

            // make sure the funds don't exceed the withdrawal limit
            let limit = ALLOWED_COIN.load(deps.storage)?;
            ensure!(
                limit.limit >= amount,
                ContractError::WithdrawalLimitExceeded {}
            );

            // Update amount
            let new_amount = account.balance - amount;

            // Update account details
            let new_details = AccountDetails {
                balance: new_amount,
                latest_withdrawal: Some(env.block.time),
            };

            // Save changes
            ACCOUNTS.save(deps.storage, info.sender.to_string(), &new_details)?;
        } else {
            // make sure the funds requested don't exceed the user's balance
            ensure!(
                account.balance >= amount,
                ContractError::InsufficientFunds {}
            );

            // make sure the funds don't exceed the withdrawal limit
            let limit = ALLOWED_COIN.load(deps.storage)?;
            ensure!(
                limit.limit >= amount,
                ContractError::WithdrawalLimitExceeded {}
            );

            // Update amount
            let new_amount = account.balance - amount;

            // Update account details
            let new_details = AccountDetails {
                balance: new_amount,
                latest_withdrawal: Some(env.block.time),
            };

            // Save changes
            ACCOUNTS.save(deps.storage, info.sender.to_string(), &new_details)?;
        }

        let coin = Coin {
            denom: ALLOWED_COIN.load(deps.storage)?.coin,
            amount,
        };

        // Transfer funds
        let res = Response::new()
            .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![coin.clone()],
            }))
            .add_attribute("action", "withdrew funds")
            .add_attribute("coin", coin.to_string());
        Ok(res)
    } else {
        Err(ContractError::AccountNotFound {})
    }
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
