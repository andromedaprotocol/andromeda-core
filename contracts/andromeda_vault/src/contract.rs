use ado_base::state::ADOContract;
use andromeda_protocol::vault::{
    self, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, StrategyType, BALANCES,
    STRATEGY_CONTRACT_ADDRESSES,
};
use common::{
    ado_base::{
        recipient::Recipient, AndromedaMsg, AndromedaQuery, InstantiateMsg as BaseInstantiateMsg,
    },
    error::ContractError,
    parse_message, require,
};
use cosmwasm_std::{
    coin, entry_point, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, SubMsg, Uint128,
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
            // Find funds in sent funds and add to current balance
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
    ADOContract::default().query(deps, env, msg, query)
}

#[cfg(test)]
mod tests {
    use super::*;
    use andromeda_protocol::vault::{InstantiateMsg, YieldStrategy, BALANCES};
    use cosmwasm_std::{
        coin,
        testing::{mock_dependencies, mock_env, mock_info},
        to_binary, wasm_execute, CosmosMsg, ReplyOn,
    };

    #[test]
    fn test_instantiate() {
        let yield_strategy = YieldStrategy {
            strategy_type: StrategyType::Anchor,
            address: "terra1anchoraddress".to_string(),
        };
        let inst_msg = InstantiateMsg {
            operators: None,
            strategies: vec![yield_strategy],
        };
        let env = mock_env();
        let info = mock_info("minter", &[]);
        let mut deps = mock_dependencies(&[]);

        instantiate(deps.as_mut(), env, info, inst_msg).unwrap();
    }

    #[test]
    fn test_deposit() {
        let env = mock_env();
        let sent_funds = coin(100, "uusd");
        let extra_sent_funds = coin(100, "uluna");
        let depositor = "depositor".to_string();
        let mut deps = mock_dependencies(&[]);

        let info = mock_info(&depositor, &[sent_funds.clone(), extra_sent_funds.clone()]);
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
    fn test_deposit_insufficient_funds() {
        let env = mock_env();
        let depositor = "depositor".to_string();
        let mut deps = mock_dependencies(&[]);

        let info = mock_info(&depositor, &[]);
        let msg = ExecuteMsg::Deposit {
            recipient: None,
            amount: None,
            strategy: None,
        };

        let err = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(ContractError::InsufficientFunds {}, err);

        let info_with_funds = mock_info(&depositor, &[coin(100, "uusd")]);
        let msg = ExecuteMsg::Deposit {
            recipient: None,
            amount: Some(coin(0u128, "uusd")),
            strategy: None,
        };

        let err = execute(deps.as_mut(), env, info_with_funds, msg).unwrap_err();
        assert_eq!(ContractError::InsufficientFunds {}, err)
    }

    #[test]
    fn test_deposit_strategy() {
        let yield_strategy = YieldStrategy {
            strategy_type: StrategyType::Anchor,
            address: "terra1anchoraddress".to_string(),
        };
        let inst_msg = InstantiateMsg {
            operators: None,
            strategies: vec![yield_strategy.clone()],
        };
        let env = mock_env();
        let info = mock_info("minter", &[]);
        let mut deps = mock_dependencies(&[]);

        instantiate(deps.as_mut(), env.clone(), info, inst_msg).unwrap();

        let sent_funds = coin(100, "uusd");
        let extra_sent_funds = coin(100, "uluna");
        let funds = vec![sent_funds.clone(), extra_sent_funds.clone()];
        let depositor = "depositor".to_string();
        let info = mock_info(&depositor, &funds);
        let msg = ExecuteMsg::Deposit {
            recipient: None,
            amount: None,
            strategy: Some(yield_strategy.strategy_type),
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let msg = wasm_execute(
            yield_strategy.address.clone(),
            &ExecuteMsg::AndrReceive(AndromedaMsg::Receive(Some(
                to_binary(&"depositor".to_string()).unwrap(),
            ))),
            vec![sent_funds],
        )
        .unwrap();
        let msg_two = wasm_execute(
            yield_strategy.address,
            &ExecuteMsg::AndrReceive(AndromedaMsg::Receive(Some(
                to_binary(&"depositor".to_string()).unwrap(),
            ))),
            vec![extra_sent_funds],
        )
        .unwrap();
        let deposit_submsg = SubMsg {
            id: 1,
            msg: CosmosMsg::Wasm(msg),
            gas_limit: None,
            reply_on: ReplyOn::Error,
        };
        let deposit_submsg_two = SubMsg {
            id: 1,
            msg: CosmosMsg::Wasm(msg_two),
            gas_limit: None,
            reply_on: ReplyOn::Error,
        };
        let expected = Response::default()
            .add_submessage(deposit_submsg)
            .add_submessage(deposit_submsg_two);

        assert_eq!(expected, res)
    }

    #[test]
    fn test_deposit_strategy_partial_amount() {
        let yield_strategy = YieldStrategy {
            strategy_type: StrategyType::Anchor,
            address: "terra1anchoraddress".to_string(),
        };
        let inst_msg = InstantiateMsg {
            operators: None,
            strategies: vec![yield_strategy.clone()],
        };
        let env = mock_env();
        let info = mock_info("minter", &[]);
        let mut deps = mock_dependencies(&[]);

        instantiate(deps.as_mut(), env.clone(), info, inst_msg).unwrap();

        let sent_funds = coin(90, "uusd");
        let funds = vec![sent_funds.clone()];
        BALANCES
            .save(
                deps.as_mut().storage,
                ("depositor".to_string(), sent_funds.denom.clone()),
                &Uint128::from(20u128),
            )
            .unwrap();

        let depositor = "depositor".to_string();
        let info = mock_info(&depositor, &funds);
        let msg = ExecuteMsg::Deposit {
            recipient: None,
            amount: Some(coin(100, sent_funds.denom.clone())),
            strategy: Some(yield_strategy.strategy_type),
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let msg = wasm_execute(
            yield_strategy.address.clone(),
            &ExecuteMsg::AndrReceive(AndromedaMsg::Receive(Some(
                to_binary(&"depositor".to_string()).unwrap(),
            ))),
            vec![coin(100, sent_funds.denom.clone())],
        )
        .unwrap();
        let deposit_submsg = SubMsg {
            id: 1,
            msg: CosmosMsg::Wasm(msg),
            gas_limit: None,
            reply_on: ReplyOn::Error,
        };
        let expected = Response::default().add_submessage(deposit_submsg);

        assert_eq!(expected, res);

        let post_balance = BALANCES
            .load(
                deps.as_ref().storage,
                ("depositor".to_string(), sent_funds.denom.clone()),
            )
            .unwrap();

        assert_eq!(Uint128::from(10u128), post_balance);
    }

    #[test]
    fn test_deposit_strategy_empty_funds_non_empty_amount() {
        let env = mock_env();
        let mut deps = mock_dependencies(&[]);

        let depositor = "depositor".to_string();
        let info = mock_info(&depositor, &[]);
        let msg = ExecuteMsg::Deposit {
            recipient: None,
            amount: Some(coin(100, "uusd")),
            strategy: None,
        };

        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

        assert_eq!(ContractError::InsufficientFunds {}, err);
    }

    #[test]
    fn test_deposit_strategy_insufficient_partial_amount() {
        let yield_strategy = YieldStrategy {
            strategy_type: StrategyType::Anchor,
            address: "terra1anchoraddress".to_string(),
        };
        let inst_msg = InstantiateMsg {
            operators: None,
            strategies: vec![yield_strategy.clone()],
        };
        let env = mock_env();
        let info = mock_info("minter", &[]);
        let mut deps = mock_dependencies(&[]);

        instantiate(deps.as_mut(), env.clone(), info, inst_msg).unwrap();

        let sent_funds = coin(90, "uusd");
        let funds = vec![sent_funds.clone()];
        BALANCES
            .save(
                deps.as_mut().storage,
                ("depositor".to_string(), sent_funds.denom.clone()),
                &Uint128::from(5u128),
            )
            .unwrap();

        let depositor = "depositor".to_string();
        let info = mock_info(&depositor, &funds);
        let msg = ExecuteMsg::Deposit {
            recipient: None,
            amount: Some(coin(100, sent_funds.denom.clone())),
            strategy: Some(yield_strategy.strategy_type),
        };

        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(ContractError::InsufficientFunds {}, err);

        let post_balance = BALANCES
            .load(
                deps.as_ref().storage,
                ("depositor".to_string(), sent_funds.denom.clone()),
            )
            .unwrap();

        assert_eq!(Uint128::from(5u128), post_balance);
    }
}
