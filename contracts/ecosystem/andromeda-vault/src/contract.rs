use ado_base::state::ADOContract;
use amp::messages::AMPPkt;
use andromeda_ecosystem::vault::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, StrategyAddressResponse, StrategyType,
    BALANCES, STRATEGY_CONTRACT_ADDRESSES,
};
use common::{
    ado_base::{
        operators::IsOperatorResponse, recipient::Recipient, AndromedaMsg, AndromedaQuery,
        InstantiateMsg as BaseInstantiateMsg, QueryMsg as AndrQueryMsg,
    },
    app::AndrAddress,
    encode_binary,
    error::ContractError,
    parse_message,
    withdraw::{Withdrawal, WithdrawalType},
};
use cosmwasm_std::{
    coin, ensure, entry_point, from_binary, to_binary, BankMsg, Binary, Coin, ContractResult,
    CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Order, QueryRequest, Reply, ReplyOn,
    Response, StdError, SubMsg, SystemResult, Uint128, WasmMsg, WasmQuery,
};
use cw2::{get_contract_version, set_contract_version};
use cw_utils::nonpayable;
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-vault";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "vault".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: None,
            primitive_contract: None,
            kernel_address: msg.kernel_address,
        },
    )
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
    match msg {
        ExecuteMsg::AndrReceive(msg) => execute_andr_receive(deps, env, info, msg),
        ExecuteMsg::Deposit {
            recipient,
            amount,
            strategy,
        } => execute_deposit(deps, env, info, amount, recipient, strategy),
        ExecuteMsg::Withdraw {
            recipient,
            withdrawals,
            strategy,
        } => execute_withdraw(deps, env, info, recipient, withdrawals, strategy),
        ExecuteMsg::UpdateStrategy { strategy, address } => {
            execute_update_strategy(deps, env, info, strategy, address)
        }
        ExecuteMsg::AMPReceive(pkt) => handle_amp_packet(deps, env, info, pkt),
    }
}

fn execute_andr_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AndromedaMsg,
) -> Result<Response, ContractError> {
    match msg {
        AndromedaMsg::Receive(None) => {
            let sender = info.sender.to_string();
            execute_deposit(deps, env, info, None, Some(Recipient::Addr(sender)), None)
        }
        _ => ADOContract::default().execute(deps, env, info, msg, execute),
    }
}

pub struct ExecuteEnv<'a> {
    deps: DepsMut<'a>,
    pub env: Env,
    pub info: MessageInfo,
}

fn handle_amp_packet(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    packet: AMPPkt,
) -> Result<Response, ContractError> {
    let mut res = Response::default();
    let execute_env = ExecuteEnv { deps, env, info };

    let msg_opt = packet.messages.first();
    if let Some(msg) = msg_opt {
        let exec_msg: ExecuteMsg = from_binary(&msg.message)?;
        let funds = msg.funds.to_vec();
        let mut exec_info = execute_env.info.clone();
        exec_info.funds = funds;

        let exec_res = execute(
            execute_env.deps,
            execute_env.env.clone(),
            exec_info,
            exec_msg,
        )?;

        res = res
            .add_attributes(exec_res.attributes)
            .add_submessages(exec_res.messages)
            .add_events(exec_res.events);
    }

    Ok(res)
}

fn execute_deposit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Option<Coin>,
    recipient: Option<Recipient>,
    strategy: Option<StrategyType>,
) -> Result<Response, ContractError> {
    let mut resp = Response::default();
    let recipient = recipient.unwrap_or_else(|| Recipient::Addr(info.sender.to_string()));

    // If no amount is provided then the sent funds are used as a deposit
    let deposited_funds = if let Some(deposit_amount) = amount {
        ensure!(
            !deposit_amount.amount.is_zero(),
            ContractError::InsufficientFunds {}
        );
        let mut deposit_balance = info
            .funds
            .iter()
            .find(|f| f.denom == deposit_amount.denom.clone())
            .unwrap_or(&Coin {
                denom: deposit_amount.denom.clone(),
                amount: Uint128::zero(),
            })
            .amount;

        // If depositing to a yield strategy we must first check that the sender has either provided the amount to deposit
        // or has a combination of the amount within the vault and within the sent funds
        if strategy.is_some() && deposit_balance <= deposit_amount.amount {
            let recipient_addr = recipient.get_addr(
                deps.api,
                &deps.querier,
                ADOContract::default().get_app_contract(deps.storage)?,
            )?;
            let balance_key = (recipient_addr.as_str(), deposit_amount.denom.as_str());
            let vault_balance = BALANCES
                .may_load(deps.storage, balance_key)?
                .unwrap_or_else(Uint128::zero);

            // Amount that must be removed from the vault to add to the deposit
            let difference = deposit_amount.amount.checked_sub(deposit_balance)?;
            ensure!(
                vault_balance >= difference,
                ContractError::InsufficientFunds {}
            );

            deposit_balance = deposit_balance.checked_add(vault_balance)?;
            //Subtract the removed funds from balance
            BALANCES.save(
                deps.storage,
                balance_key,
                &vault_balance.checked_sub(difference)?,
            )?;
        }

        // Ensure enough funds are present
        ensure!(
            deposit_balance >= deposit_amount.amount,
            ContractError::InsufficientFunds {}
        );
        let funds_vec: Vec<Coin> = vec![deposit_amount];
        funds_vec
    } else {
        info.funds
    };

    ensure!(
        !deposited_funds.is_empty(),
        ContractError::InsufficientFunds {}
    );
    match strategy {
        // Depositing to vault
        None => {
            for funds in deposited_funds {
                let recipient_addr = recipient.get_addr(
                    deps.api,
                    &deps.querier,
                    ADOContract::default().get_app_contract(deps.storage)?,
                )?;
                let balance_key = (recipient_addr.as_str(), funds.denom.as_str());
                let curr_balance = BALANCES
                    .may_load(deps.storage, balance_key)?
                    .unwrap_or_default();
                BALANCES.save(
                    deps.storage,
                    balance_key,
                    &curr_balance.checked_add(funds.amount)?,
                )?;
            }
        }
        Some(strategy) => {
            let mut deposit_msgs: Vec<SubMsg> = Vec::new();
            for funds in deposited_funds {
                let deposit_msg = strategy.deposit(deps.storage, funds, recipient.clone())?;
                deposit_msgs.push(deposit_msg);
            }
            resp = resp.add_submessages(deposit_msgs)
        }
    }

    Ok(resp)
}

pub fn execute_withdraw(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: Option<Recipient>,
    withdrawals: Vec<Withdrawal>,
    strategy: Option<StrategyType>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    ensure!(
        !withdrawals.is_empty(),
        ContractError::InvalidTokensToWithdraw {
            msg: "No tokens provided for withdrawal".to_string(),
        }
    );
    match strategy {
        None => withdraw_vault(deps, info, recipient, withdrawals),
        Some(strategy) => withdraw_strategy(deps, info, strategy, withdrawals),
    }
}

pub fn withdraw_vault(
    deps: DepsMut,
    info: MessageInfo,
    recipient: Option<Recipient>,
    withdrawals: Vec<Withdrawal>,
) -> Result<Response, ContractError> {
    let res = Response::default();
    let mut withdrawal_amount: Vec<Coin> = Vec::new();

    let recipient = recipient
        .unwrap_or_else(|| Recipient::Addr(info.sender.to_string()))
        .get_addr(
            deps.api,
            &deps.querier,
            ADOContract::default().get_app_contract(deps.storage)?,
        )?;
    for withdrawal in withdrawals {
        let denom = withdrawal.token;
        let balance = BALANCES
            .load(deps.storage, (info.sender.as_str(), &denom))
            .unwrap_or_else(|_| Uint128::zero());
        ensure!(!balance.is_zero(), ContractError::InsufficientFunds {});

        match withdrawal.withdrawal_type {
            Some(withdrawal_type) => match withdrawal_type {
                WithdrawalType::Amount(amount) => {
                    ensure!(
                        !amount.is_zero(),
                        ContractError::InvalidWithdrawal {
                            msg: Some("Amount must be non-zero".to_string()),
                        }
                    );
                    ensure!(balance >= amount, ContractError::InsufficientFunds {});
                    withdrawal_amount.push(coin(amount.u128(), denom.clone()));
                    BALANCES.save(
                        deps.storage,
                        (info.sender.as_str(), &denom),
                        &balance.checked_sub(amount)?,
                    )?;
                }
                WithdrawalType::Percentage(percent) => {
                    ensure!(
                        !percent.is_zero(),
                        ContractError::InvalidWithdrawal {
                            msg: Some("Percent must be non-zero".to_string()),
                        }
                    );
                    let amount = balance * percent;
                    withdrawal_amount.push(coin(amount.u128(), denom.clone()));
                    BALANCES.save(
                        deps.storage,
                        (info.sender.as_str(), &denom),
                        &balance.checked_sub(amount)?,
                    )?;
                }
            },
            None => {
                withdrawal_amount.push(coin(balance.u128(), denom.clone()));
                BALANCES.save(
                    deps.storage,
                    (info.sender.as_str(), &denom),
                    &Uint128::zero(),
                )?;
            }
        }
    }
    let withdrawal_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: recipient,
        amount: withdrawal_amount,
    });
    Ok(res.add_message(withdrawal_msg))
}

pub fn withdraw_strategy(
    deps: DepsMut,
    info: MessageInfo,
    strategy: StrategyType,
    withdrawals: Vec<Withdrawal>,
) -> Result<Response, ContractError> {
    let res = Response::default();
    let recipient = Recipient::Addr(info.sender.to_string());
    let addr_opt = STRATEGY_CONTRACT_ADDRESSES.may_load(deps.storage, strategy.to_string())?;
    if addr_opt.is_none() {
        return Err(ContractError::InvalidStrategy {
            strategy: strategy.to_string(),
        });
    }

    let addr = addr_opt.unwrap();
    let withdraw_exec = to_binary(&ExecuteMsg::AndrReceive(AndromedaMsg::Withdraw {
        recipient: Some(recipient),
        tokens_to_withdraw: Some(withdrawals),
    }))?;
    let withdraw_submsg = SubMsg {
        id: 104,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: addr,
            msg: withdraw_exec,
            funds: vec![],
        }),
        gas_limit: None,
        reply_on: ReplyOn::Error,
    };

    Ok(res.add_submessage(withdraw_submsg))
}

fn execute_update_strategy(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    strategy: StrategyType,
    address: AndrAddress,
) -> Result<Response, ContractError> {
    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_ref())?,
        ContractError::Unauthorized {}
    );
    let app_contract = ADOContract::default().get_app_contract(deps.storage)?;
    let strategy_addr = address.get_address(deps.api, &deps.querier, app_contract)?;

    //The vault contract must be an operator for the given contract in order to enable withdrawals
    //DEV: with custom approval functionality this check can be removed
    // let strategy_is_operator: IsOperatorResponse = query_get(
    //     Some(to_binary(&AndromedaQuery::IsOperator {
    //         address: env.contract.address.to_string(),
    //     })?),
    //     strategy_addr.clone(),
    //     &deps.querier,
    // )?;
    let strategy_is_operator: IsOperatorResponse = deps.querier.query_wasm_smart(
        strategy_addr.clone(),
        &QueryMsg::AndrQuery(AndromedaQuery::IsOperator {
            address: env.contract.address.to_string(),
        }),
    )?;
    ensure!(
        strategy_is_operator.is_operator,
        ContractError::NotAssignedOperator {
            msg: Some("Vault contract is not an operator for the given address".to_string()),
        }
    );

    STRATEGY_CONTRACT_ADDRESSES.save(deps.storage, strategy.to_string(), &strategy_addr)?;

    Ok(Response::default()
        .add_attribute("action", "update_strategy")
        .add_attribute("strategy_type", strategy.to_string())
        .add_attribute("addr", strategy_addr))
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

fn from_semver(err: semver::Error) -> StdError {
    StdError::generic_err(format!("Semver: {}", err))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => handle_andromeda_query(deps, env, msg),
        QueryMsg::Balance {
            address,
            strategy,
            denom,
        } => query_balance(deps, env, address, strategy, denom),
        QueryMsg::StrategyAddress { strategy } => query_strategy_address(deps, env, strategy),
    }
}

fn handle_andromeda_query(
    deps: Deps,
    env: Env,
    msg: AndromedaQuery,
) -> Result<Binary, ContractError> {
    match msg {
        AndromedaQuery::Get(data) => {
            let address: String = parse_message(&data)?;
            encode_binary(&query_balance(deps, env, address, None, None)?)
        }
        _ => ADOContract::default().query(deps, env, msg, query),
    }
}

fn query_balance(
    deps: Deps,
    _env: Env,
    address: String,
    strategy: Option<StrategyType>,
    denom: Option<String>,
) -> Result<Binary, ContractError> {
    if let Some(strategy) = strategy {
        let strategy_addr = STRATEGY_CONTRACT_ADDRESSES.load(deps.storage, strategy.to_string())?;
        // DEV NOTE: Why does this ensure! a generic type when not using custom query?
        let query: QueryRequest<Empty> = QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: strategy_addr,
            msg: to_binary(&AndrQueryMsg::AndrQuery(AndromedaQuery::Get(Some(
                to_binary(&address)?,
            ))))?,
        });
        match deps.querier.raw_query(&to_binary(&query)?) {
            SystemResult::Ok(ContractResult::Ok(value)) => Ok(value),
            _ => Err(ContractError::InvalidQuery {}),
        }
    } else if let Some(denom) = denom {
        let balance = BALANCES.load(deps.storage, (&address, denom.as_str()))?;
        Ok(to_binary(&[Coin {
            denom,
            amount: balance,
        }])?)
    } else {
        let balances: Result<Vec<Coin>, ContractError> = BALANCES
            .prefix(&address)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|v| {
                let (denom, balance) = v?;
                Ok(Coin {
                    denom,
                    amount: balance,
                })
            })
            .collect();
        Ok(to_binary(&balances?)?)
    }
}

fn query_strategy_address(
    deps: Deps,
    _env: Env,
    strategy: StrategyType,
) -> Result<Binary, ContractError> {
    let addr = STRATEGY_CONTRACT_ADDRESSES.may_load(deps.storage, strategy.to_string())?;
    match addr {
        Some(addr) => Ok(to_binary(&StrategyAddressResponse {
            address: addr,
            strategy,
        })?),
        None => Err(ContractError::InvalidStrategy {
            strategy: strategy.to_string(),
        }),
    }
}
