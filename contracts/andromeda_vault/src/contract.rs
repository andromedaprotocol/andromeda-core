use ado_base::state::ADOContract;
use andromeda_protocol::vault::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, StrategyAddressResponse, StrategyType,
    BALANCES, STRATEGY_CONTRACT_ADDRESSES,
};
use common::{
    ado_base::{
        recipient::Recipient, AndromedaMsg, AndromedaQuery, InstantiateMsg as BaseInstantiateMsg,
        QueryMsg as AndrQueryMsg,
    },
    error::ContractError,
    parse_message, require,
    withdraw::{Withdrawal, WithdrawalType},
};
use cosmwasm_std::{
    coin, entry_point, to_binary, BankMsg, Binary, Coin, ContractResult, CosmosMsg, Deps, DepsMut,
    Empty, Env, MessageInfo, Order, QueryRequest, ReplyOn, Response, SubMsg, SystemResult, Uint128,
    WasmMsg, WasmQuery,
};
use cw2::{get_contract_version, set_contract_version};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-rates";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    msg.validate()?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    for strategy in msg.strategies {
        STRATEGY_CONTRACT_ADDRESSES.save(
            deps.storage,
            strategy.strategy_type.to_string(),
            &strategy.address,
        )?;
    }
    ADOContract::default().instantiate(
        deps.storage,
        deps.api,
        &deps.querier,
        info,
        BaseInstantiateMsg {
            ado_type: "vault".to_string(),
            operators: msg.operators,
            modules: None,
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
    }
}

fn execute_andr_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AndromedaMsg,
) -> Result<Response, ContractError> {
    match msg {
        AndromedaMsg::Receive(data) => {
            let strategy: Option<StrategyType> = parse_message(&data)?;
            let sender = info.sender.to_string();
            execute_deposit(
                deps,
                env,
                info,
                None,
                Some(Recipient::Addr(sender)),
                strategy,
            )
        }
        _ => ADOContract::default().execute(deps, env, info, msg, execute),
    }
}

fn execute_deposit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Option<Coin>,
    recipient: Option<Recipient>,
    strategy: Option<StrategyType>,
) -> Result<Response, ContractError> {
    require(
        !info.funds.is_empty()
            && (amount.is_none()
                || (amount.is_some() && amount.clone().unwrap().amount.gt(&Uint128::zero()))),
        ContractError::InsufficientFunds {},
    )?;
    let mut resp = Response::default();
    let recipient = recipient.unwrap_or_else(|| Recipient::Addr(info.sender.to_string()));

    // If no amount is provided then the sent funds are used as a deposit
    let deposited_funds = if let Some(deposit_amount) = amount {
        let mut deposit_balance = Uint128::zero();
        for funds in info.funds {
            // Find funds in sent funds and add to deposit amount
            if funds.denom == deposit_amount.denom {
                deposit_balance = deposit_balance.checked_add(funds.amount)?;
            }
        }

        // If depositing to a yield strategy we must first check that the sender has either provided the amount to deposit or has a combination of the amount within the vault and within the sent funds
        if strategy.is_some() && deposit_balance <= deposit_amount.amount {
            let vault_balance = BALANCES
                .may_load(
                    deps.storage,
                    (recipient.get_addr(), deposit_amount.denom.clone()),
                )?
                .unwrap_or_else(Uint128::zero);

            // Amount that must be removed from the vault to add to the deposit
            let difference = deposit_amount.amount.checked_sub(deposit_balance)?;
            require(
                vault_balance >= difference,
                ContractError::InsufficientFunds {},
            )?;

            deposit_balance = deposit_balance.checked_add(vault_balance)?;
            //Subtract the removed funds from balance
            BALANCES.save(
                deps.storage,
                (recipient.get_addr(), deposit_amount.denom.clone()),
                &vault_balance.checked_sub(difference)?,
            )?;
        }

        // Ensure enough funds are present
        require(
            deposit_balance >= deposit_amount.amount,
            ContractError::InsufficientFunds {},
        )?;
        let funds_vec: Vec<Coin> = vec![deposit_amount];
        funds_vec
    } else {
        info.funds
    };

    match strategy {
        // Depositing to vault
        None => {
            for funds in deposited_funds {
                let curr_balance = BALANCES
                    .may_load(deps.storage, (recipient.get_addr(), funds.denom.clone()))?
                    .unwrap_or_default();
                BALANCES.save(
                    deps.storage,
                    (recipient.get_addr(), funds.denom),
                    &curr_balance.checked_add(funds.amount)?,
                )?;
            }
        }
        Some(strategy) => {
            let mut deposit_msgs: Vec<SubMsg> = Vec::new();
            for funds in deposited_funds {
                let deposit_msg = strategy.deposit(deps.storage, funds, &recipient.get_addr())?;
                // resp = resp.add_submessage(sub_msg)
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
    require(
        !withdrawals.is_empty(),
        ContractError::InvalidTokensToWithdraw {
            msg: "No tokens provided for withdrawal".to_string(),
        },
    )?;
    match strategy {
        None => withdraw_vault(deps, info, recipient, withdrawals),
        Some(strategy) => withdraw_strategy(deps, info, strategy, recipient, withdrawals),
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
        .get_addr();
    for withdrawal in withdrawals {
        let denom = withdrawal.token;
        let balance = BALANCES
            .load(deps.storage, (info.sender.to_string(), denom.clone()))
            .unwrap_or_else(|_| Uint128::zero());
        require(!balance.is_zero(), ContractError::InsufficientFunds {})?;

        match withdrawal.withdrawal_type {
            Some(withdrawal_type) => match withdrawal_type {
                WithdrawalType::Amount(amount) => {
                    require(
                        !amount.is_zero(),
                        ContractError::InvalidWithdrawal {
                            msg: Some("Amount must be non-zero".to_string()),
                        },
                    )?;
                    require(balance >= amount, ContractError::InsufficientFunds {})?;
                    withdrawal_amount.push(coin(amount.u128(), denom.clone()));
                    BALANCES.save(
                        deps.storage,
                        (info.sender.to_string(), denom),
                        &balance.checked_sub(amount)?,
                    )?;
                }
                WithdrawalType::Percentage(percent) => {
                    require(
                        !percent.is_zero(),
                        ContractError::InvalidWithdrawal {
                            msg: Some("Percent must be non-zero".to_string()),
                        },
                    )?;
                    let amount = balance.multiply_ratio(percent, 100u128);
                    withdrawal_amount.push(coin(amount.u128(), denom.clone()));
                    BALANCES.save(
                        deps.storage,
                        (info.sender.to_string(), denom),
                        &balance.checked_sub(amount)?,
                    )?;
                }
            },
            None => {
                withdrawal_amount.push(coin(balance.u128(), denom.clone()));
                BALANCES.save(
                    deps.storage,
                    (info.sender.to_string(), denom),
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
    recipient: Option<Recipient>,
    withdrawals: Vec<Withdrawal>,
) -> Result<Response, ContractError> {
    let res = Response::default();
    let recipient = recipient.unwrap_or_else(|| Recipient::Addr(info.sender.to_string()));
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }
    Ok(Response::default())
}

#[entry_point]
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
    ADOContract::default().query(deps, env, msg, query)
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
        // DEV NOTE: Why does this require a generic type when not using custom query?
        let query: QueryRequest<Empty> = QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: strategy_addr,
            msg: to_binary(&AndrQueryMsg::AndrQuery(AndromedaQuery::Get(Some(
                to_binary(&address)?,
            ))))?,
        });
        match deps.querier.raw_query(&to_binary(&query)?.to_vec()) {
            SystemResult::Ok(ContractResult::Ok(value)) => Ok(value),
            _ => Err(ContractError::InvalidQuery {}),
        }
    } else if let Some(denom) = denom {
        let balance = BALANCES.load(deps.storage, (address, denom.clone()))?;
        Ok(to_binary(&[Coin {
            denom,
            amount: balance,
        }])?)
    } else {
        let balances: Vec<Coin> = BALANCES
            .prefix(address)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|v| {
                let (denom_vec, balance) = v.unwrap();
                let denom = String::from_utf8(denom_vec).unwrap();
                Coin {
                    denom,
                    amount: balance,
                }
            })
            .collect();
        Ok(to_binary(&balances)?)
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
