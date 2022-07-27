use crate::state::{ACCOUNTS, ALLOWED_COIN};
use ado_base::ADOContract;
use andromeda_finance::rate_limiting_withdrawals::{
    AccountDetails, CoinAllowance, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use common::{
    ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError, require,
};
use cosmwasm_std::{
    entry_point, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, Uint128,
};
use cw2::{get_contract_version, set_contract_version};

use cw_utils::{nonpayable, one_coin};
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-splitter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    ALLOWED_COIN.save(deps.storage, &msg.allowed_coin)?;
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
        ExecuteMsg::Deposit { recipient } => execute_deposit(deps, env, info, recipient),
        ExecuteMsg::Withdraw { amount } => execute_withdraw(deps, env, info, amount),
        ExecuteMsg::AndrReceive(msg) => {
            ADOContract::default().execute(deps, env, info, msg, execute)
        }
        ExecuteMsg::UpdateAllowedCoin { new_coin } => {
            execute_update_allowed_coin(deps, env, info, new_coin)
        }
    }
}

pub fn execute_update_allowed_coin(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_coin: CoinAllowance,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let contract = ADOContract::default();

    // Only owner or operator can call this function
    require(
        contract.is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

    ALLOWED_COIN.save(deps.storage, &new_coin)?;
    Ok(Response::new()
        .add_attribute("action", "updated allowed coin")
        .add_attribute("new_coin", new_coin.coin)
        .add_attribute("new_withdrawal_limit", new_coin.limit)
        .add_attribute(
            "new_minimal_withdrawal_frequency",
            new_coin.minimal_withdrawal_frequency.to_string(),
        ))
}

pub fn execute_deposit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    // Only one coin at a time
    one_coin(&info)?;

    // Coin has to be in the allowed list
    let coin = ALLOWED_COIN.load(deps.storage)?;
    require(
        coin.coin == info.funds[0].denom,
        ContractError::InvalidFunds {
            msg: "Coin must be part of the allowed list".to_string(),
        },
    )?;

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

pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    // check if sender has an account
    let account = ACCOUNTS.may_load(deps.storage, info.sender.to_string())?;
    if let Some(account) = account {
        // Calculate time since last withdrawal
        if let Some(latest_withdrawal) = account.latest_withdrawal {
            let minimum_withdrawal_frequency = ALLOWED_COIN
                .load(deps.storage)?
                .minimal_withdrawal_frequency;
            let current_time = env.block.time.seconds();
            let seconds_since_withdrawal = current_time - latest_withdrawal.seconds();

            // make sure enough time has elapsed since the latest withdrawal
            require(
                seconds_since_withdrawal >= minimum_withdrawal_frequency,
                ContractError::FundsAreLocked {},
            )?;

            // make sure the funds requested don't exceed the user's balance
            require(
                account.balance >= amount,
                ContractError::InsufficientFunds {},
            )?;

            // make sure the funds don't exceed the withdrawal limit
            let limit = ALLOWED_COIN.load(deps.storage)?;
            require(
                limit.limit >= amount,
                ContractError::WithdrawalLimitExceeded {},
            )?;

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
            require(
                account.balance >= amount,
                ContractError::InsufficientFunds {},
            )?;

            // make sure the funds don't exceed the withdrawal limit
            let limit = ALLOWED_COIN.load(deps.storage)?;
            require(
                limit.limit >= amount,
                ContractError::WithdrawalLimitExceeded {},
            )?;

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
        QueryMsg::CoinAllowanceDetails {} => encode_binary(&query_coin_allowance_details(deps)?),
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

fn query_coin_allowance_details(deps: Deps) -> Result<CoinAllowance, ContractError> {
    let details = ALLOWED_COIN.load(deps.storage)?;
    Ok(details)
}

#[cfg(test)]
mod tests {
    use super::*;
    use andromeda_finance::rate_limiting_withdrawals::CoinAllowance;
    use cosmwasm_std::coin;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    // 1 day in seconds
    const ONE_DAY: u64 = 86_400;

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            modules: None,
            allowed_coin: CoinAllowance {
                coin: "junox".to_string(),
                limit: Uint128::from(50_u64),
                minimal_withdrawal_frequency: ONE_DAY,
            },
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_update_allowed_coin_unauthorized() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            modules: None,
            allowed_coin: CoinAllowance {
                coin: "junox".to_string(),
                limit: Uint128::from(50_u64),
                minimal_withdrawal_frequency: ONE_DAY,
            },
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        let new_coin = CoinAllowance {
            coin: "juno".to_string(),
            limit: Uint128::from(10_u64),
            minimal_withdrawal_frequency: 600,
        };
        let info = mock_info("random", &[]);
        let msg = ExecuteMsg::UpdateAllowedCoin { new_coin };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {})
    }

    #[test]
    fn test_update_allowed_coin_works() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            modules: None,
            allowed_coin: CoinAllowance {
                coin: "junox".to_string(),
                limit: Uint128::from(50_u64),
                minimal_withdrawal_frequency: ONE_DAY,
            },
        };
        let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());
        let new_coin = CoinAllowance {
            coin: "juno".to_string(),
            limit: Uint128::from(10_u64),
            minimal_withdrawal_frequency: 600,
        };
        let msg = ExecuteMsg::UpdateAllowedCoin { new_coin };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let expected_allowed_coin = CoinAllowance {
            coin: "juno".to_string(),
            limit: Uint128::from(10_u64),
            minimal_withdrawal_frequency: 600,
        };
        let allowed_coin = ALLOWED_COIN.load(&deps.storage).unwrap();
        assert_eq!(expected_allowed_coin, allowed_coin)
    }

    #[test]
    fn test_deposit_zero_funds() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            modules: None,
            allowed_coin: CoinAllowance {
                coin: "junox".to_string(),
                limit: Uint128::from(50_u64),
                minimal_withdrawal_frequency: ONE_DAY,
            },
        };
        let _res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();

        let exec = ExecuteMsg::Deposit { recipient: None };
        let _err = execute(deps.as_mut(), mock_env(), info, exec).unwrap_err();
    }

    #[test]
    fn test_deposit_invalid_funds() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            modules: None,
            allowed_coin: CoinAllowance {
                coin: "junox".to_string(),
                limit: Uint128::from(50_u64),
                minimal_withdrawal_frequency: ONE_DAY,
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        let exec = ExecuteMsg::Deposit {
            recipient: Some("me".to_string()),
        };

        let info = mock_info("creator", &[coin(30, "uusd")]);

        let err = execute(deps.as_mut(), mock_env(), info, exec).unwrap_err();
        assert_eq!(
            err,
            ContractError::InvalidFunds {
                msg: "Coin must be part of the allowed list".to_string(),
            }
        )
    }

    #[test]
    fn test_deposit_new_account_works() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            modules: None,
            allowed_coin: CoinAllowance {
                coin: "junox".to_string(),
                limit: Uint128::from(50_u64),
                minimal_withdrawal_frequency: ONE_DAY,
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        let exec = ExecuteMsg::Deposit {
            recipient: Some("andromedauser".to_string()),
        };

        let info = mock_info("creator", &[coin(30, "junox")]);

        let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();
        let expected_balance = AccountDetails {
            balance: Uint128::from(30_u16),
            latest_withdrawal: None,
        };
        let actual_balance = ACCOUNTS
            .load(&deps.storage, "andromedauser".to_string())
            .unwrap();
        assert_eq!(expected_balance, actual_balance)
    }

    #[test]
    fn test_deposit_existing_account_works() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            modules: None,
            allowed_coin: CoinAllowance {
                coin: "junox".to_string(),
                limit: Uint128::from(50_u64),
                minimal_withdrawal_frequency: ONE_DAY,
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        let exec = ExecuteMsg::Deposit {
            recipient: Some("andromedauser".to_string()),
        };

        let info = mock_info("creator", &[coin(30, "junox")]);

        let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();
        let exec = ExecuteMsg::Deposit { recipient: None };

        let info = mock_info("andromedauser", &[coin(70, "junox")]);

        let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();
        let expected_balance = AccountDetails {
            balance: Uint128::from(100_u16),
            latest_withdrawal: None,
        };
        let actual_balance = ACCOUNTS
            .load(&deps.storage, "andromedauser".to_string())
            .unwrap();
        assert_eq!(expected_balance, actual_balance)
    }

    #[test]
    fn test_withdraw_account_not_found() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            modules: None,
            allowed_coin: CoinAllowance {
                coin: "junox".to_string(),
                limit: Uint128::from(50_u64),
                minimal_withdrawal_frequency: ONE_DAY,
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        let exec = ExecuteMsg::Deposit {
            recipient: Some("andromedauser".to_string()),
        };

        let info = mock_info("creator", &[coin(30, "junox")]);

        let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

        let info = mock_info("random", &[]);
        let exec = ExecuteMsg::Withdraw {
            amount: Uint128::from(19_u16),
        };
        let err = execute(deps.as_mut(), mock_env(), info, exec).unwrap_err();
        assert_eq!(err, ContractError::AccountNotFound {});
    }

    #[test]
    fn test_withdraw_over_account_limit() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            modules: None,
            allowed_coin: CoinAllowance {
                coin: "junox".to_string(),
                limit: Uint128::from(50_u64),
                minimal_withdrawal_frequency: ONE_DAY,
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        let exec = ExecuteMsg::Deposit {
            recipient: Some("andromedauser".to_string()),
        };

        let info = mock_info("creator", &[coin(30, "junox")]);

        let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

        let info = mock_info("andromedauser", &[]);
        let exec = ExecuteMsg::Withdraw {
            amount: Uint128::from(31_u16),
        };
        let err = execute(deps.as_mut(), mock_env(), info, exec).unwrap_err();
        assert_eq!(err, ContractError::InsufficientFunds {});
    }

    #[test]
    fn test_withdraw_funds_locked() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            modules: None,
            allowed_coin: CoinAllowance {
                coin: "junox".to_string(),
                limit: Uint128::from(50_u64),
                minimal_withdrawal_frequency: ONE_DAY,
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        let exec = ExecuteMsg::Deposit {
            recipient: Some("andromedauser".to_string()),
        };

        let info = mock_info("creator", &[coin(30, "junox")]);

        let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

        let info = mock_info("andromedauser", &[]);
        let exec = ExecuteMsg::Withdraw {
            amount: Uint128::from(10_u16),
        };
        let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

        let info = mock_info("andromedauser", &[]);
        let exec = ExecuteMsg::Withdraw {
            amount: Uint128::from(10_u16),
        };

        let err = execute(deps.as_mut(), mock_env(), info, exec).unwrap_err();

        assert_eq!(err, ContractError::FundsAreLocked {});
    }

    #[test]
    fn test_withdraw_over_allowed_limit() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            modules: None,
            allowed_coin: CoinAllowance {
                coin: "junox".to_string(),
                limit: Uint128::from(20_u64),
                minimal_withdrawal_frequency: ONE_DAY,
            },
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        let exec = ExecuteMsg::Deposit {
            recipient: Some("andromedauser".to_string()),
        };

        let info = mock_info("creator", &[coin(30, "junox")]);

        let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

        let info = mock_info("andromedauser", &[]);
        let exec = ExecuteMsg::Withdraw {
            amount: Uint128::from(21_u16),
        };
        let err = execute(deps.as_mut(), mock_env(), info, exec).unwrap_err();
        assert_eq!(err, ContractError::WithdrawalLimitExceeded {});
    }

    #[test]
    fn test_withdraw_works() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            modules: None,
            allowed_coin: CoinAllowance {
                coin: "junox".to_string(),
                limit: Uint128::from(50_u64),
                minimal_withdrawal_frequency: ONE_DAY,
            },
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        let exec = ExecuteMsg::Deposit {
            recipient: Some("andromedauser".to_string()),
        };

        let info = mock_info("creator", &[coin(30, "junox")]);

        let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

        let info = mock_info("andromedauser", &[]);
        let exec = ExecuteMsg::Withdraw {
            amount: Uint128::from(10_u16),
        };
        let _res = execute(deps.as_mut(), mock_env(), info, exec).unwrap();

        let expected_balance = AccountDetails {
            balance: Uint128::from(20_u16),
            latest_withdrawal: Some(env.block.time),
        };
        let actual_balance = ACCOUNTS
            .load(&deps.storage, "andromedauser".to_string())
            .unwrap();
        assert_eq!(expected_balance, actual_balance)
    }
}
