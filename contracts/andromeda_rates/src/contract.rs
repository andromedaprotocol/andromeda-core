use crate::state::{Config, CONFIG};
use andromeda_protocol::{
    communication::{encode_binary, parse_message, AndromedaMsg, AndromedaQuery},
    error::ContractError,
    modules::common::{calculate_fee, deduct_funds},
    operators::{execute_update_operators, query_is_operator, query_operators},
    ownership::{execute_update_owner, is_contract_owner, query_contract_owner, CONTRACT_OWNER},
    rates::{
        DeductedFundsResponse, ExecuteMsg, InstantiateMsg, PaymentsResponse, QueryMsg, RateInfo,
    },
    require,
};
use cosmwasm_std::{
    attr, entry_point, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, SubMsg,
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config { rates: msg.rates };
    CONTRACT_OWNER.save(deps.storage, &info.sender)?;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attributes(vec![attr("action", "instantiate"), attr("type", "rates")]))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AndrReceive(msg) => execute_andr_receive(deps, info, msg),
        ExecuteMsg::UpdateRates { rates } => execute_update_rates(deps, info, rates),
    }
}

fn execute_andr_receive(
    deps: DepsMut,
    info: MessageInfo,
    msg: AndromedaMsg,
) -> Result<Response, ContractError> {
    match msg {
        AndromedaMsg::Receive(data) => {
            let rates: Vec<RateInfo> = parse_message(data)?;
            execute_update_rates(deps, info, rates)
        }
        AndromedaMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
        AndromedaMsg::UpdateOperators { operators } => {
            execute_update_operators(deps, info, operators)
        }
    }
}

fn execute_update_rates(
    deps: DepsMut,
    info: MessageInfo,
    rates: Vec<RateInfo>,
) -> Result<Response, ContractError> {
    require(
        is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    let mut config = CONFIG.load(deps.storage)?;
    config.rates = rates;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_rates")]))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => handle_andromeda_query(deps, msg),
        QueryMsg::Payments {} => encode_binary(&query_payments(deps)?),
    }
}

fn handle_andromeda_query(deps: Deps, msg: AndromedaQuery) -> Result<Binary, ContractError> {
    match msg {
        AndromedaQuery::Get(data) => {
            let coin: Coin = parse_message(data)?;
            encode_binary(&query_deducted_funds(deps, coin)?)
        }
        AndromedaQuery::Owner {} => encode_binary(&query_contract_owner(deps)?),
        AndromedaQuery::Operators {} => encode_binary(&query_operators(deps)?),
        AndromedaQuery::IsOperator { address } => {
            encode_binary(&query_is_operator(deps, &address)?)
        }
    }
}

fn query_payments(deps: Deps) -> Result<PaymentsResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let rates = config.rates;

    Ok(PaymentsResponse { payments: rates })
}

fn query_deducted_funds(deps: Deps, coin: Coin) -> Result<DeductedFundsResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut deducted_funds = vec![coin];
    let mut msgs: Vec<SubMsg> = vec![];
    for rate_info in config.rates.iter() {
        // TODO: Figure out what rate_info.is_additive does.
        let rate = rate_info.rate.validate(&deps.querier)?;
        let fee = calculate_fee(rate, &deducted_funds[0])?;
        deduct_funds(&mut deducted_funds, &fee)?;
        for reciever in rate_info.receivers.iter() {
            msgs.push(reciever.generate_msg(&deps, vec![fee.clone()])?);
        }
    }
    Ok(DeductedFundsResponse { msgs })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{execute, instantiate, query};
    use andromeda_protocol::{
        communication::{encode_binary, AndromedaMsg, AndromedaQuery, Recipient},
        modules::{ADORate, Rate},
        rates::{InstantiateMsg, PaymentsResponse, QueryMsg, RateInfo},
        testing::mock_querier::{mock_dependencies_custom, MOCK_PRIMITIVE_CONTRACT},
    };
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, from_binary, BankMsg, Coin, CosmosMsg, Uint128};

    #[test]
    fn test_instantiate_query() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "owner";
        let info = mock_info(owner, &[]);
        let rates = vec![
            RateInfo {
                rate: Rate::Percent(10u128.into()),
                is_additive: true,
                description: Some("desc1".to_string()),
                receivers: vec![Recipient::Addr("".into())],
            },
            RateInfo {
                rate: Rate::Flat(Coin {
                    amount: Uint128::from(10u128),
                    denom: "uusd".to_string(),
                }),
                is_additive: false,
                description: Some("desc2".to_string()),
                receivers: vec![Recipient::Addr("".into())],
            },
        ];
        let msg = InstantiateMsg {
            rates: rates.clone(),
        };
        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        assert_eq!(0, res.messages.len());

        let payments = query(deps.as_ref(), env, QueryMsg::Payments {}).unwrap();

        assert_eq!(
            payments,
            encode_binary(&PaymentsResponse { payments: rates }).unwrap()
        );

        //Why does this test error?
        //let payments = query(deps.as_ref(), mock_env(), QueryMsg::Payments {}).is_err();
        //assert_eq!(payments, true);
    }

    #[test]
    fn test_andr_receive() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "owner";
        let info = mock_info(owner, &[]);
        let rates = vec![
            RateInfo {
                rate: Rate::Percent(10u128.into()),
                is_additive: true,
                description: Some("desc1".to_string()),
                receivers: vec![Recipient::Addr("".into())],
            },
            RateInfo {
                rate: Rate::Flat(Coin {
                    amount: Uint128::from(10u128),
                    denom: "uusd".to_string(),
                }),
                is_additive: false,
                description: Some("desc2".to_string()),
                receivers: vec![Recipient::Addr("".into())],
            },
        ];
        let msg = InstantiateMsg { rates: vec![] };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg =
            ExecuteMsg::AndrReceive(AndromedaMsg::Receive(Some(encode_binary(&rates).unwrap())));

        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(
            Response::new().add_attributes(vec![attr("action", "update_rates")]),
            res
        );
    }

    #[test]
    fn test_query_deducted_funds() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let owner = "owner";
        let info = mock_info(owner, &[]);
        let rates = vec![
            RateInfo {
                rate: Rate::Percent(10u128.into()),
                is_additive: true,
                description: Some("desc1".to_string()),
                receivers: vec![Recipient::Addr("1".into())],
            },
            RateInfo {
                rate: Rate::Flat(Coin {
                    amount: Uint128::from(20u128),
                    denom: "uusd".to_string(),
                }),
                is_additive: false,
                description: Some("desc2".to_string()),
                receivers: vec![Recipient::Addr("2".into())],
            },
            RateInfo {
                rate: Rate::External(ADORate {
                    address: MOCK_PRIMITIVE_CONTRACT.into(),
                    key: Some("flat".into()),
                }),
                is_additive: false,
                description: Some("desc3".to_string()),
                receivers: vec![Recipient::Addr("3".into())],
            },
        ];
        let msg = InstantiateMsg {
            rates: rates.clone(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let res: DeductedFundsResponse = from_binary(
            &query(
                deps.as_ref(),
                env,
                QueryMsg::AndrQuery(AndromedaQuery::Get(Some(
                    encode_binary(&coin(100, "uusd")).unwrap(),
                ))),
            )
            .unwrap(),
        )
        .unwrap();

        let expected_msgs: Vec<SubMsg> = vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "1".into(),
                amount: coins(10, "uusd"),
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "2".into(),
                amount: coins(20, "uusd"),
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "3".into(),
                amount: coins(1, "uusd"),
            })),
        ];
        assert_eq!(expected_msgs, res.msgs);
    }
}
