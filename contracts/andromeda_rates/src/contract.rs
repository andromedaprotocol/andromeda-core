use crate::msg::{ExecuteMsg, InstantiateMsg, PaymentInfo, PaymentsResponse, QueryMsg, RateInfo};
use crate::state::{Config, CONFIG};
use andromeda_protocol::modules::common::calculate_fee;
use cosmwasm_std::{
    attr, entry_point, to_binary, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult,
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        owner: info.sender.to_string(),
        rates: msg.rates,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attributes(vec![attr("action", "instantiate"), attr("type", "rates")]))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateRates { rates } => execute_update_rates(deps, info, rates),
    }
}

fn execute_update_rates(
    deps: DepsMut,
    info: MessageInfo,
    rates: Vec<RateInfo>,
) -> StdResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender.to_string() {
        return Err(StdError::generic_err("unauthorized"));
    }
    config.rates = rates;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_rates")]))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Payments { amount } => to_binary(&query_payments(deps, amount)?),
    }
}

fn query_payments(deps: Deps, amount: Coin) -> StdResult<PaymentsResponse> {
    let mut res = vec![];
    let config = CONFIG.load(deps.storage)?;
    let rates = config.rates;

    for rate in rates.iter() {
        let fee_coin = calculate_fee(rate.rate.clone(), amount.clone());
        let result = if rate.is_additive {
            Coin {
                denom: fee_coin.denom.clone(),
                amount: amount.amount.checked_add(fee_coin.amount.clone())?,
            }
        } else {
            Coin {
                denom: fee_coin.denom.clone(),
                amount: amount.amount.checked_sub(fee_coin.amount.clone())?,
            }
        };

        res.push(PaymentInfo {
            result,
            fee: fee_coin,
            is_additive: rate.is_additive,
            description: rate.description.clone(),
        });
    }

    Ok(PaymentsResponse { payments: res })
}

#[cfg(test)]
mod tests {
    use crate::contract::{instantiate, query};
    use crate::msg::{InstantiateMsg, PaymentInfo, PaymentsResponse, QueryMsg, RateInfo};
    use andromeda_protocol::modules::{FlatRate, Rate};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, to_binary, Uint128};

    #[test]
    fn test_instantiate_query() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "owner";
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg {
            rates: vec![
                RateInfo {
                    rate: Rate::Percent(10),
                    is_additive: true,
                    description: Some("desc1".to_string()),
                },
                RateInfo {
                    rate: Rate::Flat(FlatRate {
                        amount: Uint128::from(10u128),
                        denom: "uusd".to_string(),
                    }),
                    is_additive: false,
                    description: Some("desc2".to_string()),
                },
            ], //Rate::Percent(10), Rate::Flat(FlatRate { amount: Uint128::from(10u128), denom: "uusd".to_string() })],
               //is_additive: true,
               //description: "desc1".to_string()
        };
        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        assert_eq!(0, res.messages.len());

        let payments = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Payments {
                amount: coin(100u128, "uusd".to_string()),
            },
        )
        .unwrap();

        assert_eq!(
            payments,
            to_binary(&PaymentsResponse {
                payments: vec![
                    PaymentInfo {
                        result: coin(110, "uusd".to_string()),
                        fee: coin(10, "uusd".to_string()),
                        is_additive: true,
                        description: Some("desc1".to_string())
                    },
                    PaymentInfo {
                        result: coin(90, "uusd".to_string()),
                        fee: coin(10, "uusd".to_string()),
                        is_additive: false,
                        description: Some("desc2".to_string())
                    },
                ]
            })
            .unwrap()
        );

        let payments = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Payments {
                amount: coin(5u128, "uusd".to_string()),
            },
        )
        .is_err();
        assert_eq!(payments, true);
    }
}
