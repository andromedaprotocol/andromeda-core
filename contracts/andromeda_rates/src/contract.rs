use crate::msg::{ExecuteMsg, InstantiateMsg, PaymentsResponse, QueryMsg, RateInfo};
use crate::state::{Config, CONFIG};
use cosmwasm_std::{
    attr, entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult,
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
        QueryMsg::Payments {} => to_binary(&query_payments(deps)?),
    }
}

fn query_payments(deps: Deps) -> StdResult<PaymentsResponse> {
    let config = CONFIG.load(deps.storage)?;
    let rates = config.rates;

    Ok(PaymentsResponse { payments: rates })
}

#[cfg(test)]
mod tests {
    use crate::contract::{instantiate, query};
    use crate::msg::{InstantiateMsg, PaymentsResponse, QueryMsg, RateInfo};
    use andromeda_protocol::modules::{FlatRate, Rate};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{to_binary, Addr, Uint128};

    #[test]
    fn test_instantiate_query() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "owner";
        let info = mock_info(owner, &[]);
        let rates = vec![
            RateInfo {
                rate: Rate::Percent(10),
                is_additive: true,
                description: Some("desc1".to_string()),
                receivers: vec![Addr::unchecked("")],
            },
            RateInfo {
                rate: Rate::Flat(FlatRate {
                    amount: Uint128::from(10u128),
                    denom: "uusd".to_string(),
                }),
                is_additive: false,
                description: Some("desc2".to_string()),
                receivers: vec![Addr::unchecked("")],
            },
        ];
        let msg = InstantiateMsg {
            rates: rates.clone(),
        };
        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        assert_eq!(0, res.messages.len());

        let payments = query(deps.as_ref(), env.clone(), QueryMsg::Payments {}).unwrap();

        assert_eq!(
            payments,
            to_binary(&PaymentsResponse { payments: rates }).unwrap()
        );

        //Why does this test error?
        // let payments = query(deps.as_ref(), env.clone(), QueryMsg::Payments {}).is_err();
        // assert_eq!(payments, true);
    }
}
