use std::{f32::MIN, ops::Index};

use crate::state::{ACCOUNTS, ALLOWED_COINS, MINIMUM_WITHDRAWAL_FREQUENCY};
use ado_base::ADOContract;
use andromeda_finance::rate_limiting_withdrawals::{
    validate_recipient_list, AccountDetails, AddressPercent, ExecuteMsg, InstantiateMsg,
    MigrateMsg, QueryMsg,
};
use common::{
    ado_base::{
        hooks::AndromedaHook, recipient::Recipient, AndromedaMsg,
        InstantiateMsg as BaseInstantiateMsg,
    },
    app::AndrAddress,
    encode_binary,
    error::ContractError,
    require,
};
use cosmwasm_std::{
    entry_point, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, SubMsg, Timestamp, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20ReceiveMsg;
use cw_utils::{nonpayable, one_coin, Expiration};
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-splitter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
// 1 day in seconds
const ONE_DAY: u64 = 86_400;
// 1 year in seconds
const ONE_YEAR: u64 = 31_536_000;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    MINIMUM_WITHDRAWAL_FREQUENCY.save(deps.storage, &msg.minimum_withdrawal_time)?;

    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "rate-limiting-withdrawals".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: msg.modules,
            primitive_contract: None,
        },
    )
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit {} => execute_deposit(deps, env, info),
        ExecuteMsg::Withdraw { coin } => execute_withdraw(deps, env, info, coin),
        ExecuteMsg::AndrReceive(msg) => {
            ADOContract::default().execute(deps, env, info, msg, execute)
        }
    }
}

pub fn execute_deposit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Only one coin at a time
    one_coin(&info)?;

    // Coin has to be in the allowed list
    let coin = ALLOWED_COINS.may_load(deps.storage, &info.funds[0].denom)?;
    require(
        coin.is_some(),
        ContractError::InvalidFunds {
            msg: "Coin must be part of the allowed list".to_string(),
        },
    )?;
    // Load list of accounts
    let account = ACCOUNTS.may_load(deps.storage, info.sender.to_string())?;
    // Check if sender already has an account
    if let Some(mut account) = account {
        // Check the coin's index
        let coin_index = account
            .balance
            .iter()
            .position(|x| x.denom == info.funds[0].denom);
        // If the user does have an account in that coin
        if let Some(coin_index) = coin_index {
            // Calculate new amount of coins
            let new_amount = account.balance[coin_index].amount + info.funds[0].amount;
            // Create updated coin
            let updated_coin = Coin {
                denom: info.funds[0].denom.to_string(),
                amount: new_amount,
            };
            // remove old balance
            account.balance.swap_remove(coin_index);
            // add new balance with updated coin
            account.balance.push(updated_coin);
            // save changes
            ACCOUNTS.save(deps.storage, info.sender.to_string(), &account)?;

        // If user doesn't have an account with that coin
        } else {
            let new_coin = Coin {
                denom: info.funds[0].denom.to_string(),
                amount: info.funds[0].amount,
            };
            account.balance.push(new_coin);
            // save changes
            ACCOUNTS.save(deps.storage, info.sender.to_string(), &account)?;
        }
        // If user doesn't have an account at all
    } else {
        let new_user = info.sender.to_string();
        let new_coin = Coin {
            denom: info.funds[0].denom.to_string(),
            amount: info.funds[0].amount,
        };
        let new_account_details = AccountDetails {
            balance: vec![new_coin],
            latest_withdrawal: None,
        };
        // save changes
        ACCOUNTS.save(deps.storage, new_user, &new_account_details)?;
    }

    let res = Response::new()
        .add_attribute("action", "funded account")
        .add_attribute("account", info.sender.to_string());
    Ok(res)
}

pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    coin: Coin,
) -> Result<Response, ContractError> {
    // check if sender has an account
    let user = ACCOUNTS.may_load(deps.storage, info.sender.to_string())?;
    if let Some(mut user) = user {
        // Calculate time since last withdrawal
        if let Some(latest_withdrawal) = user.latest_withdrawal {
            let minimum_withdrawal_frequency = MINIMUM_WITHDRAWAL_FREQUENCY.load(deps.storage)?;
            let current_time = env.block.time.seconds();
            let seconds_since_withdrawal = current_time - latest_withdrawal.seconds();
            // make sure enough time has elapsed since the latest withdrawal
            require(
                seconds_since_withdrawal >= minimum_withdrawal_frequency,
                ContractError::FundsAreLocked {},
            )?;
            // make sure the user has the requested coin
            let coin_index = user.balance.iter().position(|x| x.denom == coin.denom);
            require(
                coin_index.is_some(),
                ContractError::InvalidFunds {
                    msg: "you don't have a balance in the requested coin".to_string(),
                },
            )?;
            // make sure the funds requested don't exceed the user's balance
            if let Some(coin_index) = coin_index {
                require(
                    user.balance[coin_index].amount >= coin.amount,
                    ContractError::InsufficientFunds {},
                )?;
                // make sure the funds don't exceed the withdrawal limit
                let limit = ALLOWED_COINS.load(deps.storage, &coin.denom)?;
                require(
                    limit >= coin.amount,
                    ContractError::WithdrawalLimitExceeded {},
                )?;
                // Update amount
                let new_amount = user.balance[coin_index].amount - coin.amount;

                // Updated coin
                let updated_coin = Coin {
                    denom: info.funds[0].denom.to_string(),
                    amount: new_amount,
                };

                // Remove old balance
                user.balance.swap_remove(coin_index);

                // Insert latest balance
                user.balance.push(updated_coin);

                // Update account details
                let new_details = AccountDetails {
                    balance: user.balance,
                    latest_withdrawal: Some(env.block.time),
                };

                // Save changes
                ACCOUNTS.save(deps.storage, info.sender.to_string(), &new_details)?;
            }
        } else {
            // make sure the user has the requested coin
            let coin_index = user.balance.iter().position(|x| x.denom == coin.denom);
            require(
                coin_index.is_some(),
                ContractError::InvalidFunds {
                    msg: "you don't have a balance in the requested coin".to_string(),
                },
            )?;
            // make sure the funds requested don't exceed the user's balance
            if let Some(coin_index) = coin_index {
                require(
                    user.balance[coin_index].amount >= coin.amount,
                    ContractError::InsufficientFunds {},
                )?;
                // make sure the funds don't exceed the withdrawal limit
                let limit = ALLOWED_COINS.load(deps.storage, &coin.denom)?;
                require(
                    limit >= coin.amount,
                    ContractError::WithdrawalLimitExceeded {},
                )?;
                // Update amount
                let new_amount = user.balance[coin_index].amount - coin.amount;

                // Updated coin
                let updated_coin = Coin {
                    denom: info.funds[0].denom.to_string(),
                    amount: new_amount,
                };

                // Remove old balance
                user.balance.swap_remove(coin_index);

                // Insert latest balance
                user.balance.push(updated_coin);

                // Update account details
                let new_details = AccountDetails {
                    balance: user.balance,
                    latest_withdrawal: Some(env.block.time),
                };

                // Save changes
                ACCOUNTS.save(deps.storage, info.sender.to_string(), &new_details)?;
            }
        }
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

    let contract = ADOContract::default();

    require(
        stored.contract == CONTRACT_NAME,
        ContractError::CannotMigrate {
            previous_contract: stored.contract,
        },
    )?;

    // New version has to be newer/greater than the old version
    require(
        storage_version < version,
        ContractError::CannotMigrate {
            previous_contract: stored.version,
        },
    )?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Update the ADOContract's version
    contract.execute_update_version(deps)?;

    Ok(Response::default())
}

fn from_semver(err: semver::Error) -> StdError {
    StdError::generic_err(format!("Semver: {}", err))
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::MinimalWithdrawalFrequency {} => {
            encode_binary(&query_minimal_withdrawal_frequency(deps)?)
        }
        QueryMsg::CoinWithdrawalLimit { coin } => {
            encode_binary(&query_coin_withdrawal_limit(deps, coin)?)
        }
        QueryMsg::AccountDetails { account } => {
            encode_binary(&query_account_details(deps, account)?)
        }
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
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

fn query_coin_withdrawal_limit(deps: Deps, coin: String) -> Result<Uint128, ContractError> {
    let limit = ALLOWED_COINS.may_load(deps.storage, &coin)?;
    if let Some(limit) = limit {
        Ok(limit)
    } else {
        Err(ContractError::CoinNotFound {})
    }
}

fn query_minimal_withdrawal_frequency(deps: Deps) -> Result<u64, ContractError> {
    let minimal_withdrawal_frequency = MINIMUM_WITHDRAWAL_FREQUENCY.load(deps.storage)?;
    Ok(minimal_withdrawal_frequency)
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::ado_base::recipient::Recipient;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_binary, Coin, Decimal};

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            minimum_withdrawal_time: ONE_DAY,
            modules: None,
            allowed_coins: vec![Coin {
                denom: "junox".to_string(),
                amount: Uint128::from(20_u32),
            }],
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_receive_zero_funds() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            minimum_withdrawal_time: ONE_DAY,
            modules: None,
            allowed_coins: vec![Coin {
                denom: "junox".to_string(),
                amount: Uint128::from(20_u32),
            }],
        };
        let _res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();

        let exec = ExecuteMsg::Deposit {};
        let err = execute(deps.as_mut(), mock_env(), info, exec).unwrap_err();
        assert_eq!(
            err,
            ContractError::InvalidFunds {
                msg: "can't send 0 funds".to_string(),
            }
        )
    }

    #[test]
    fn test_receive_invalid_funds() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            minimum_withdrawal_time: ONE_DAY,
            modules: None,
            allowed_coins: vec![Coin {
                denom: "junox".to_string(),
                amount: Uint128::from(20_u32),
            }],
        };
        let _res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
        let exec = ExecuteMsg::Deposit {};
        let err = execute(deps.as_mut(), mock_env(), info, exec).unwrap_err();
        assert_eq!(
            err,
            ContractError::InvalidFunds {
                msg: "Coin must be part of the allowed list".to_string(),
            }
        )
    }
}
