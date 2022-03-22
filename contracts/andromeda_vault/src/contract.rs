use crate::state::CONFIG;
use ado_base::state::ADOContract;
use andromeda_protocol::vault::{
    Config, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, StrategyType, BALANCES,
    STRATEGY_CONTRACT_ADDRESSES,
};
use common::{
    ado_base::{
        recipient::Recipient, AndromedaMsg, AndromedaQuery, InstantiateMsg as BaseInstantiateMsg,
    },
    error::ContractError,
    parse_message, require,
};
use cosmwasm_std::{entry_point, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, Uint128};
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
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config {
        default_yield_strategy: msg.default_yield_strategy.strategy_type.clone(),
    };
    CONFIG.save(deps.storage, &config)?;
    STRATEGY_CONTRACT_ADDRESSES.save(
        deps.storage,
        msg.default_yield_strategy.strategy_type.to_string(),
        &msg.default_yield_strategy.address,
    )?;
    ADOContract::default().instantiate(
        deps.storage,
        deps.api,
        &deps.querier,
        info,
        BaseInstantiateMsg {
            ado_type: "vault".to_string(),
            operators: None,
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
    require(info.funds.len() > 0, ContractError::InsufficientFunds {})?;
    let mut resp = Response::default();
    let recipient = recipient.unwrap_or_else(|| Recipient::Addr(info.sender.to_string()));
    // If no amount is provided then the sent funds are used as a deposit
    let deposited_funds = if let Some(deposit_amount) = amount {
        // If depositing to a yield strategy we must first check that the sender has either provided the amount to deposit or has a combination of the amount within the vault and within the sent funds
        if strategy.is_some() {
            // Fetch balance from vault
            let mut current_balance = BALANCES
                .may_load(
                    deps.storage,
                    (recipient.get_addr(), deposit_amount.denom.clone()),
                )?
                .unwrap_or(Uint128::zero());
            for funds in info.funds {
                // Find funds in sent funds and add to current balance
                if funds.denom == deposit_amount.denom {
                    current_balance = current_balance.checked_add(funds.amount)?;
                }
            }
            // Ensure enough funds are present
            require(
                current_balance >= deposit_amount.amount,
                ContractError::InsufficientFunds {},
            )?;
        }
        let funds_vec: Vec<Coin> = vec![deposit_amount];
        funds_vec
    } else {
        info.funds
    };

    match strategy {
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
            for funds in deposited_funds {
                let sub_msg = strategy.deposit(deps.storage, funds, &recipient.get_addr())?;
                resp = resp.add_submessage(sub_msg)
            }
        }
    }

    Ok(resp)
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
    }
}

fn handle_andromeda_query(
    deps: Deps,
    env: Env,
    msg: AndromedaQuery,
) -> Result<Binary, ContractError> {
    match msg {
        _ => ADOContract::default().query(deps, env, msg, query),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use andromeda_protocol::vault::{InstantiateMsg, YieldStrategy, BALANCES};
    use cosmwasm_std::{
        coin,
        testing::{mock_dependencies, mock_env, mock_info},
    };

    #[test]
    fn test_instantiate() {
        let yield_strategy = YieldStrategy {
            strategy_type: StrategyType::Anchor,
            address: "terra1anchoraddress".to_string(),
        };
        let inst_msg = InstantiateMsg {
            default_yield_strategy: yield_strategy.clone(),
        };
        let env = mock_env();
        let info = mock_info("minter", &[]);
        let mut deps = mock_dependencies(&[]);

        instantiate(deps.as_mut(), env, info, inst_msg).unwrap();

        let config = CONFIG.load(deps.as_ref().storage).unwrap();

        assert_eq!(config.default_yield_strategy, yield_strategy.strategy_type)
    }

    #[test]
    fn test_deposit() {
        let env = mock_env();
        let sent_funds = coin(100, "uusd");
        let extra_sent_funds = coin(100, "uluna");
        let depositor = "depositor".to_string();
        let mut deps = mock_dependencies(&[]);

        let info = mock_info(
            &depositor,
            &vec![sent_funds.clone(), extra_sent_funds.clone()],
        );
        let msg = ExecuteMsg::Deposit {
            recipient: None,
            amount: None,
            strategy: None,
        };

        execute(deps.as_mut(), env, info, msg).unwrap();

        let uusd_balance = BALANCES
            .load(
                deps.as_ref().storage,
                (depositor.clone(), "uusd".to_string()),
            )
            .unwrap();
        assert_eq!(uusd_balance, sent_funds.amount);
        let uluna_balance = BALANCES
            .load(deps.as_ref().storage, (depositor, "uluna".to_string()))
            .unwrap();
        assert_eq!(uluna_balance, extra_sent_funds.amount)
    }

    #[test]
    fn test_deposit_invalid_funds() {}
}
