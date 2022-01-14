use crate::state::{Config, CONFIG};
use andromeda_protocol::{
    communication::{encode_binary, parse_message, AndromedaMsg, AndromedaQuery},
    error::ContractError,
    modules::Rate,
    operators::{execute_update_operators, query_operators},
    ownership::{execute_update_owner, is_contract_owner, query_contract_owner, CONTRACT_OWNER},
    primitive::{
        ExecuteMsg as PrimitiveExecuteMsg, GetValueResponse, Primitive,
        QueryMsg as PrimitiveQueryMsg,
    },
    rates::{ExecuteMsg, InstantiateMsg, PaymentsResponse, QueryMsg, RateInfo, RateResponse},
    require,
};
use cosmwasm_std::{
    attr, entry_point, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QueryRequest, Response,
    Uint128, WasmMsg, WasmQuery,
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        rates: msg.rates,
        primitive_contract: deps.api.addr_validate(&msg.primitive_contract)?,
    };
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
        ExecuteMsg::UpdateRateData { name, rate } => {
            execute_update_rate_data(deps, info, name, rate)
        }
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

fn execute_update_rate_data(
    deps: DepsMut,
    info: MessageInfo,
    name: Option<String>,
    rate: Rate,
) -> Result<Response, ContractError> {
    require(
        is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    let value: Primitive = match rate {
        Rate::Percent(percent) => Primitive::Uint128(Uint128::from(percent)),
        Rate::Flat(coin) => Primitive::Coin(coin.clone()),
    };
    let config = CONFIG.load(deps.storage)?;
    let execute_msg = WasmMsg::Execute {
        contract_addr: config.primitive_contract.to_string(),
        funds: info.funds,
        msg: encode_binary(&PrimitiveExecuteMsg::SetValue { name, value })?,
    };
    Ok(Response::new().add_messages(vec![CosmosMsg::Wasm(execute_msg)]))
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
        QueryMsg::Rate { name } => encode_binary(&query_rate(deps, name)?),
    }
}

fn handle_andromeda_query(deps: Deps, msg: AndromedaQuery) -> Result<Binary, ContractError> {
    match msg {
        AndromedaQuery::Get(_) => encode_binary(&query_payments(deps)?),
        AndromedaQuery::Owner {} => encode_binary(&query_contract_owner(deps)?),
        AndromedaQuery::Operators {} => encode_binary(&query_operators(deps)?),
    }
}

fn query_rate(deps: Deps, name: Option<String>) -> Result<RateResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let response: GetValueResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.primitive_contract.to_string(),
        msg: encode_binary(&PrimitiveQueryMsg::GetValue { name: name.clone() })?,
    }))?;
    let rate: Rate = match response.value {
        Primitive::Coin(coin) => Rate::Flat(coin),
        Primitive::Uint128(value) => Rate::Percent(value.u128()),
        _ => {
            return Err(ContractError::ParsingError {
                err: "Stored rate is not a coin or Uint128".to_string(),
            })
        }
    };
    Ok(RateResponse { name, rate })
}

fn query_payments(deps: Deps) -> Result<PaymentsResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let rates = config.rates;

    Ok(PaymentsResponse { payments: rates })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{execute, instantiate, query};
    use andromeda_protocol::{
        communication::{encode_binary, AndromedaMsg, AndromedaQuery},
        modules::Rate,
        rates::{InstantiateMsg, PaymentsResponse, QueryMsg, RateInfo},
    };
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{Addr, Coin, Uint128};

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
                rate: Rate::Flat(Coin {
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
            primitive_contract: "primitive_contract".to_string(),
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
                rate: Rate::Percent(10),
                is_additive: true,
                description: Some("desc1".to_string()),
                receivers: vec![Addr::unchecked("")],
            },
            RateInfo {
                rate: Rate::Flat(Coin {
                    amount: Uint128::from(10u128),
                    denom: "uusd".to_string(),
                }),
                is_additive: false,
                description: Some("desc2".to_string()),
                receivers: vec![Addr::unchecked("")],
            },
        ];
        let msg = InstantiateMsg {
            rates: vec![],
            primitive_contract: "primitive_contract".to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg =
            ExecuteMsg::AndrReceive(AndromedaMsg::Receive(Some(encode_binary(&rates).unwrap())));

        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(
            Response::new().add_attributes(vec![attr("action", "update_rates")]),
            res
        );

        let msg = QueryMsg::AndrQuery(AndromedaQuery::Get(None));

        let payments = query(deps.as_ref(), env, msg).unwrap();

        assert_eq!(
            payments,
            encode_binary(&PaymentsResponse { payments: rates }).unwrap()
        );
    }
}
