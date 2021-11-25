use cosmwasm_std::{attr, entry_point, to_binary, Binary, Deps, DepsMut, Env,
                   MessageInfo, Response, StdError, StdResult,  Coin};
use andromeda_protocol::modules::common::calculate_fee;
use andromeda_protocol::modules::Rate;
use crate::msg::{ExecuteMsg, InstantiateMsg, PaymentsResponse, QueryMsg};
use crate::state::{
    Config, CONFIG,
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config{
        owner: info.sender.to_string(),
        rates: msg.rates,
        is_additive: msg.is_additive,
        description: msg.description
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attributes(vec![attr("action", "instantiate"), attr("type", "rates")]))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateRates { rates } => execute_update_rates(deps, info, rates)
    }
}

fn execute_update_rates(
    deps: DepsMut,
    info: MessageInfo,
    rates: Vec<Rate>
) -> StdResult<Response>{
    let mut config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender.to_string() {
        return Err(StdError::generic_err("unauthorized"));
    }
    config.rates = rates;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attributes(vec![
            attr("action", "update_rates")
        ])
    )
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Payments { amount } => to_binary(&query_payments(deps, amount)?),        
    }
}

fn query_payments(
    deps: Deps,
    amount: Coin,    
)->StdResult<PaymentsResponse>{
    let mut res = vec![];
    let config = CONFIG.load(deps.storage)?;
    let rates = config.rates;
    let is_additive = config.is_additive;

    for rate in rates.iter(){
        let fee_coin = calculate_fee(rate.clone(), amount.clone());
        res.push(fee_coin);
    }

    Ok(PaymentsResponse{
        payments: res,
        is_additive,
        description: config.description,
    })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, to_binary, Uint128};
    use andromeda_protocol::modules::{FlatRate, Rate};
    use crate::contract::{instantiate, query};
    use crate::msg::{InstantiateMsg, PaymentsResponse, QueryMsg};

    #[test]
    fn test_instantiate_query() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "owner";
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg {
            rates: vec![Rate::Percent(10), Rate::Flat(FlatRate { amount: Uint128::from(10u128), denom: "uusd".to_string() })],
            is_additive: true,
            description: "desc1".to_string()
        };
        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        assert_eq!(0, res.messages.len());

        let payments = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Payments { amount: coin(100u128, "uusd".to_string()) }).unwrap();
        assert_eq!(payments, to_binary(&PaymentsResponse{
            payments: vec![coin(10, "uusd".to_string()), coin(10,"uusd".to_string())],
            is_additive: true,
            description: "desc1".to_string()
        }).unwrap());

    }


}