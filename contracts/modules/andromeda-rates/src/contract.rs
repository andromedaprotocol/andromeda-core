use crate::state::{Config, CONFIG};
use ado_base::ADOContract;
use andromeda_modules::rates::{
    calculate_fee, ExecuteMsg, InstantiateMsg, MigrateMsg, PaymentAttribute, PaymentsResponse,
    QueryMsg, RateInfo,
};
use common::{
    ado_base::{
        hooks::{AndromedaHook, OnFundsTransferResponse},
        AndromedaMsg, AndromedaQuery, InstantiateMsg as BaseInstantiateMsg,
    },
    deduct_funds, encode_binary,
    error::ContractError,
    parse_message, Funds,
};
use cosmwasm_std::{
    attr, coin, ensure, entry_point, Binary, Coin, Deps, DepsMut, Env, Event, MessageInfo,
    Response, StdError, SubMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20Coin;
use cw_utils::nonpayable;
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-rates";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config { rates: msg.rates };
    CONFIG.save(deps.storage, &config)?;
    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "rates".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: None,
            primitive_contract: None,
            kernel_address: msg.kernel_address,
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
        ExecuteMsg::UpdateRates { rates } => execute_update_rates(deps, info, rates),
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
            let rates: Vec<RateInfo> = parse_message(&data)?;
            execute_update_rates(deps, info, rates)
        }
        _ => ADOContract::default().execute(deps, env, info, msg, execute),
    }
}

fn execute_update_rates(
    deps: DepsMut,
    info: MessageInfo,
    rates: Vec<RateInfo>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    let mut config = CONFIG.load(deps.storage)?;
    config.rates = rates;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_rates")]))
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
    StdError::generic_err(format!("Semver: {err}"))
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => handle_andromeda_query(deps, env, msg),
        QueryMsg::AndrHook(msg) => handle_andromeda_hook(deps, msg),
        QueryMsg::Payments {} => encode_binary(&query_payments(deps)?),
    }
}

fn handle_andromeda_query(
    deps: Deps,
    env: Env,
    msg: AndromedaQuery,
) -> Result<Binary, ContractError> {
    match msg {
        AndromedaQuery::Get(data) => {
            let funds: Funds = parse_message(&data)?;
            encode_binary(&Some(query_deducted_funds(deps, funds)?))
        }
        _ => ADOContract::default().query(deps, env, msg, query),
    }
}

fn handle_andromeda_hook(deps: Deps, msg: AndromedaHook) -> Result<Binary, ContractError> {
    match msg {
        AndromedaHook::OnFundsTransfer { amount, .. } => {
            encode_binary(&query_deducted_funds(deps, amount)?)
        }
        _ => Ok(encode_binary(&None::<Response>)?),
    }
}

fn query_payments(deps: Deps) -> Result<PaymentsResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let rates = config.rates;

    Ok(PaymentsResponse { payments: rates })
}

fn query_deducted_funds(
    deps: Deps,
    funds: Funds,
) -> Result<OnFundsTransferResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut msgs: Vec<SubMsg> = vec![];
    let mut events: Vec<Event> = vec![];
    let (coin, is_native): (Coin, bool) = match funds {
        Funds::Native(coin) => (coin, true),
        Funds::Cw20(cw20_coin) => (coin(cw20_coin.amount.u128(), cw20_coin.address), false),
    };
    let mut leftover_funds = vec![coin.clone()];
    for rate_info in config.rates.iter() {
        let event_name = if rate_info.is_additive {
            "tax"
        } else {
            "royalty"
        };
        let mut event = Event::new(event_name);
        if let Some(desc) = &rate_info.description {
            event = event.add_attribute("description", desc);
        }
        let app_contract = ADOContract::default().get_app_contract(deps.storage)?;
        let rate = rate_info
            .rate
            .validate(deps.api, &deps.querier, app_contract)?;
        let fee = calculate_fee(rate, &coin)?;
        for reciever in rate_info.recipients.iter() {
            if !rate_info.is_additive {
                deduct_funds(&mut leftover_funds, &fee)?;
                event = event.add_attribute("deducted", fee.to_string());
            }
            event = event.add_attribute(
                "payment",
                PaymentAttribute {
                    receiver: reciever.get_addr(
                        deps.api,
                        &deps.querier,
                        ADOContract::default().get_app_contract(deps.storage)?,
                    )?,
                    amount: fee.clone(),
                }
                .to_string(),
            );
            let msg = if is_native {
                reciever.generate_msg_native(
                    deps.api,
                    &deps.querier,
                    ADOContract::default().get_app_contract(deps.storage)?,
                    vec![fee.clone()],
                )?
            } else {
                reciever.generate_msg_cw20(
                    deps.api,
                    &deps.querier,
                    ADOContract::default().get_app_contract(deps.storage)?,
                    Cw20Coin {
                        amount: fee.amount,
                        address: fee.denom.to_string(),
                    },
                )?
            };
            msgs.push(msg);
        }
        events.push(event);
    }
    Ok(OnFundsTransferResponse {
        msgs,
        leftover_funds: if is_native {
            Funds::Native(leftover_funds[0].clone())
        } else {
            Funds::Cw20(Cw20Coin {
                amount: leftover_funds[0].amount,
                address: coin.denom,
            })
        },
        events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{execute, instantiate, query};
    use andromeda_modules::rates::{InstantiateMsg, PaymentsResponse, QueryMsg, Rate, RateInfo};
    use andromeda_testing::testing::mock_querier::{
        mock_dependencies_custom, MOCK_PRIMITIVE_CONTRACT,
    };
    use common::{ado_base::recipient::Recipient, encode_binary};
    use common::{app::AndrAddress, primitive::PrimitivePointer};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{
        coin, coins, from_binary, BankMsg, Coin, CosmosMsg, Decimal, Uint128, WasmMsg,
    };
    use cw20::Cw20ExecuteMsg;

    #[test]
    fn test_instantiate_query() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner = "owner";
        let info = mock_info(owner, &[]);
        let rates = vec![
            RateInfo {
                rate: Rate::from(Decimal::percent(10)),
                is_additive: true,
                description: Some("desc1".to_string()),
                recipients: vec![Recipient::Addr("".into())],
            },
            RateInfo {
                rate: Rate::Flat(Coin {
                    amount: Uint128::from(10u128),
                    denom: "uusd".to_string(),
                }),
                is_additive: false,
                description: Some("desc2".to_string()),
                recipients: vec![Recipient::Addr("".into())],
            },
        ];
        let msg = InstantiateMsg {
            rates: rates.clone(),
            kernel_address: None,
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
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner = "owner";
        let info = mock_info(owner, &[]);
        let rates = vec![
            RateInfo {
                rate: Rate::from(Decimal::percent(10)),
                is_additive: true,
                description: Some("desc1".to_string()),
                recipients: vec![Recipient::Addr("".into())],
            },
            RateInfo {
                rate: Rate::Flat(Coin {
                    amount: Uint128::from(10u128),
                    denom: "uusd".to_string(),
                }),
                is_additive: false,
                description: Some("desc2".to_string()),
                recipients: vec![Recipient::Addr("".into())],
            },
        ];
        let msg = InstantiateMsg {
            rates: vec![],
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg =
            ExecuteMsg::AndrReceive(AndromedaMsg::Receive(Some(encode_binary(&rates).unwrap())));

        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(
            Response::new().add_attributes(vec![attr("action", "update_rates")]),
            res
        );
    }

    #[test]
    fn test_query_deducted_funds_native() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let owner = "owner";
        let info = mock_info(owner, &[]);
        let rates = vec![
            RateInfo {
                rate: Rate::Flat(Coin {
                    amount: Uint128::from(20u128),
                    denom: "uusd".to_string(),
                }),
                is_additive: true,
                description: Some("desc2".to_string()),
                recipients: vec![Recipient::Addr("1".into())],
            },
            RateInfo {
                rate: Rate::from(Decimal::percent(10)),
                is_additive: false,
                description: Some("desc1".to_string()),
                recipients: vec![Recipient::Addr("2".into())],
            },
            RateInfo {
                rate: Rate::External(PrimitivePointer {
                    address: AndrAddress {
                        identifier: MOCK_PRIMITIVE_CONTRACT.to_owned(),
                    },
                    key: Some("flat".into()),
                }),
                is_additive: false,
                description: Some("desc3".to_string()),
                recipients: vec![Recipient::Addr("3".into())],
            },
        ];
        let msg = InstantiateMsg {
            rates,
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let res: OnFundsTransferResponse = from_binary(
            &query(
                deps.as_ref(),
                env,
                QueryMsg::AndrQuery(AndromedaQuery::Get(Some(
                    encode_binary(&Funds::Native(coin(100, "uusd"))).unwrap(),
                ))),
            )
            .unwrap(),
        )
        .unwrap();

        let expected_msgs: Vec<SubMsg> = vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "1".into(),
                amount: coins(20, "uusd"),
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "2".into(),
                amount: coins(10, "uusd"),
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "3".into(),
                amount: coins(1, "uusd"),
            })),
        ];

        assert_eq!(
            OnFundsTransferResponse {
                msgs: expected_msgs,
                // Deduct 10% from the percent rate, followed by flat fee of 1 from the external rate.
                leftover_funds: Funds::Native(coin(89, "uusd")),
                events: vec![
                    Event::new("tax")
                        .add_attribute("description", "desc2")
                        .add_attribute("payment", "1<20uusd"),
                    Event::new("royalty")
                        .add_attribute("description", "desc1")
                        .add_attribute("deducted", "10uusd")
                        .add_attribute("payment", "2<10uusd"),
                    Event::new("royalty")
                        .add_attribute("description", "desc3")
                        .add_attribute("deducted", "1uusd")
                        .add_attribute("payment", "3<1uusd"),
                ]
            },
            res
        );
    }

    #[test]
    fn test_query_deducted_funds_cw20() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let owner = "owner";
        let info = mock_info(owner, &[]);
        let cw20_address = "address";
        let rates = vec![
            RateInfo {
                rate: Rate::Flat(Coin {
                    amount: Uint128::from(20u128),
                    denom: cw20_address.to_string(),
                }),
                is_additive: true,
                description: Some("desc2".to_string()),
                recipients: vec![Recipient::Addr("1".into())],
            },
            RateInfo {
                rate: Rate::from(Decimal::percent(10)),
                is_additive: false,
                description: Some("desc1".to_string()),
                recipients: vec![Recipient::Addr("2".into())],
            },
            RateInfo {
                rate: Rate::External(PrimitivePointer {
                    address: AndrAddress {
                        identifier: MOCK_PRIMITIVE_CONTRACT.to_owned(),
                    },
                    key: Some("flat_cw20".to_string()),
                }),
                is_additive: false,
                description: Some("desc3".to_string()),
                recipients: vec![Recipient::Addr("3".into())],
            },
        ];
        let msg = InstantiateMsg {
            rates,
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let res: OnFundsTransferResponse = from_binary(
            &query(
                deps.as_ref(),
                env,
                QueryMsg::AndrQuery(AndromedaQuery::Get(Some(
                    encode_binary(&Funds::Cw20(Cw20Coin {
                        amount: 100u128.into(),
                        address: "address".into(),
                    }))
                    .unwrap(),
                ))),
            )
            .unwrap(),
        )
        .unwrap();

        let expected_msgs: Vec<SubMsg> = vec![
            SubMsg::new(WasmMsg::Execute {
                contract_addr: cw20_address.to_string(),
                msg: encode_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "1".to_string(),
                    amount: 20u128.into(),
                })
                .unwrap(),
                funds: vec![],
            }),
            SubMsg::new(WasmMsg::Execute {
                contract_addr: cw20_address.to_string(),
                msg: encode_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "2".to_string(),
                    amount: 10u128.into(),
                })
                .unwrap(),
                funds: vec![],
            }),
            SubMsg::new(WasmMsg::Execute {
                contract_addr: cw20_address.to_string(),
                msg: encode_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "3".to_string(),
                    amount: 1u128.into(),
                })
                .unwrap(),
                funds: vec![],
            }),
        ];
        assert_eq!(
            OnFundsTransferResponse {
                msgs: expected_msgs,
                // Deduct 10% from the percent rate, followed by flat fee of 1 from the external rate.
                leftover_funds: Funds::Cw20(Cw20Coin {
                    amount: 89u128.into(),
                    address: cw20_address.to_string()
                }),
                events: vec![
                    Event::new("tax")
                        .add_attribute("description", "desc2")
                        .add_attribute("payment", "1<20address"),
                    Event::new("royalty")
                        .add_attribute("description", "desc1")
                        .add_attribute("deducted", "10address")
                        .add_attribute("payment", "2<10address"),
                    Event::new("royalty")
                        .add_attribute("description", "desc3")
                        .add_attribute("deducted", "1address")
                        .add_attribute("payment", "3<1address"),
                ]
            },
            res
        );
    }
}
