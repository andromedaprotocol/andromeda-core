use crate::contract::{execute, instantiate, query};
use crate::state::RATES;
use crate::testing::mock_querier::{
    mock_dependencies_custom, MOCK_KERNEL_CONTRACT, MOCK_OWNER, MOCK_RECIPIENT1, MOCK_RECIPIENT2,
};
use andromeda_modules::rates::{ExecuteMsg, InstantiateMsg, QueryMsg, RateResponse};

use andromeda_std::ado_base::hooks::OnFundsTransferResponse;
use andromeda_std::ado_contract::rates::{LocalRate, LocalRateType, LocalRateValue, Rate};
use andromeda_std::amp::AndrAddr;
use andromeda_std::common::Funds;
use andromeda_std::{amp::recipient::Recipient, common::encode_binary};

use cosmwasm_std::{attr, Decimal, Event};
use cosmwasm_std::{
    coin, coins,
    testing::{mock_env, mock_info},
    BankMsg, Coin, CosmosMsg, Response, SubMsg, Uint128, WasmMsg,
};
use cw20::{Cw20Coin, Cw20ExecuteMsg};

#[test]
fn test_instantiate_query() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let owner = "owner";
    let info = mock_info(owner, &[]);
    let action = "deposit".to_string();
    let rate = Rate::Local(LocalRate {
        rate_type: LocalRateType::Additive,
        recipients: vec![Recipient {
            address: AndrAddr::from_string("owner".to_string()),
            msg: None,
            ibc_recovery_address: None,
        }],
        value: LocalRateValue::Flat(coin(100_u128, "uandr")),
        description: None,
    });
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        action: action.clone(),
        rate: rate.clone(),
    };
    let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    assert_eq!(0, res.messages.len());

    let rate_resp = query(deps.as_ref(), env, QueryMsg::Rate { action }).unwrap();

    assert_eq!(rate_resp, encode_binary(&RateResponse { rate }).unwrap());

    //Why does this test error?
    //let payments = query(deps.as_ref(), mock_env(), QueryMsg::Payments {}).is_err();
    //assert_eq!(payments, true);
}

#[test]
fn test_andr_receive() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let owner = "owner";
    let info = mock_info(owner, &[]);
    let action = "deposit".to_string();
    let rate = Rate::Local(LocalRate {
        rate_type: LocalRateType::Additive,
        recipients: vec![Recipient {
            address: AndrAddr::from_string("owner".to_string()),
            msg: None,
            ibc_recovery_address: None,
        }],
        value: LocalRateValue::Flat(coin(100_u128, "uandr")),
        description: None,
    });
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        action: action.clone(),
        rate: rate.clone(),
    };
    let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    // Update rate
    let msg = ExecuteMsg::UpdateRate {
        action: action.clone(),
        rate,
    };

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    assert_eq!(
        Response::new().add_attributes(vec![attr("action", "update_rates")]),
        res
    );
    let rate = Rate::Local(LocalRate {
        rate_type: LocalRateType::Additive,
        recipients: vec![Recipient {
            address: AndrAddr::from_string("owner".to_string()),
            msg: None,
            ibc_recovery_address: None,
        }],
        value: LocalRateValue::Flat(coin(200_u128, "uandr")),
        description: None,
    });
    // Set Rate
    let action = "withdraw".to_string();
    let msg = ExecuteMsg::SetRate {
        action: action.clone(),
        rate: rate.clone(),
    };
    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    let rates_resp = RATES.load(&deps.storage, &action).unwrap();
    assert_eq!(rates_resp, rate);

    // Remove Rate
    let action = "withdraw".to_string();
    let msg = ExecuteMsg::RemoveRate {
        action: action.clone(),
    };
    execute(deps.as_mut(), env, info, msg).unwrap();
    let rates_resp = RATES.may_load(&deps.storage, &action).unwrap();
    assert!(rates_resp.is_none())
}

// #[test]
// fn test_query_deducted_funds_native() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let env = mock_env();
//     let info = mock_info(MOCK_OWNER, &[]);
//     let rates = vec![
//         RateInfo {
//             rate: Rate::Flat(Coin {
//                 amount: Uint128::from(20u128),
//                 denom: "uusd".to_string(),
//             }),
//             is_additive: true,
//             description: Some("desc2".to_string()),
//             recipients: vec![Recipient::from_string(MOCK_RECIPIENT1)],
//         },
//         RateInfo {
//             rate: Rate::from(Decimal::percent(10)),
//             is_additive: false,
//             description: Some("desc1".to_string()),
//             recipients: vec![Recipient::from_string(MOCK_RECIPIENT2)],
//         },
//     ];
//     let msg = InstantiateMsg {
//         rates,
//         kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
//         owner: None,
//     };
//     let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();
//     let res = query_deducted_funds(deps.as_ref(), Funds::Native(coin(100, "uusd"))).unwrap();

//     let expected_msgs: Vec<SubMsg> = vec![
//         SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
//             to_address: MOCK_RECIPIENT1.into(),
//             amount: coins(20, "uusd"),
//         })),
//         SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
//             to_address: MOCK_RECIPIENT2.into(),
//             amount: coins(10, "uusd"),
//         })),
//     ];

//     assert_eq!(
//         OnFundsTransferResponse {
//             msgs: expected_msgs,
//             // Deduct 10% from the percent rate.
//             // NOTE: test is currently returning 90 instead
//             leftover_funds: Funds::Native(coin(90, "uusd")),
//             events: vec![
//                 Event::new("tax")
//                     .add_attribute("description", "desc2")
//                     .add_attribute("payment", "recipient1<20uusd"),
//                 Event::new("royalty")
//                     .add_attribute("description", "desc1")
//                     .add_attribute("deducted", "10uusd")
//                     .add_attribute("payment", "recipient2<10uusd"),
//             ]
//         },
//         res
//     );
// }

// #[test]
// fn test_query_deducted_funds_cw20() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let env = mock_env();
//     let owner = "owner";
//     let info = mock_info(owner, &[]);
//     let cw20_address = "address";
//     let rates = vec![
//         RateInfo {
//             rate: Rate::Flat(Coin {
//                 amount: Uint128::from(20u128),
//                 denom: cw20_address.to_string(),
//             }),
//             is_additive: true,
//             description: Some("desc2".to_string()),
//             recipients: vec![Recipient::new(MOCK_RECIPIENT1, None)],
//         },
//         RateInfo {
//             rate: Rate::from(Decimal::percent(10)),
//             is_additive: false,
//             description: Some("desc1".to_string()),
//             recipients: vec![Recipient::new(MOCK_RECIPIENT2, None)],
//         },
//     ];
//     let msg = InstantiateMsg {
//         rates,
//         kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
//         owner: None,
//     };
//     let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

//     let res: OnFundsTransferResponse = query_deducted_funds(
//         deps.as_ref(),
//         Funds::Cw20(Cw20Coin {
//             amount: 100u128.into(),
//             address: "address".into(),
//         }),
//     )
//     .unwrap();

//     let expected_msgs: Vec<SubMsg> = vec![
//         SubMsg::new(WasmMsg::Execute {
//             contract_addr: cw20_address.to_string(),
//             msg: encode_binary(&Cw20ExecuteMsg::Transfer {
//                 recipient: MOCK_RECIPIENT1.to_string(),
//                 amount: 20u128.into(),
//             })
//             .unwrap(),
//             funds: vec![],
//         }),
//         SubMsg::new(WasmMsg::Execute {
//             contract_addr: cw20_address.to_string(),
//             msg: encode_binary(&Cw20ExecuteMsg::Transfer {
//                 recipient: MOCK_RECIPIENT2.to_string(),
//                 amount: 10u128.into(),
//             })
//             .unwrap(),
//             funds: vec![],
//         }),
//     ];
//     assert_eq!(
//         OnFundsTransferResponse {
//             msgs: expected_msgs,
//             // Deduct 10% from the percent rate.
//             leftover_funds: Funds::Cw20(Cw20Coin {
//                 amount: 90u128.into(),
//                 address: cw20_address.to_string()
//             }),
//             events: vec![
//                 Event::new("tax")
//                     .add_attribute("description", "desc2")
//                     .add_attribute("payment", "recipient1<20address"),
//                 Event::new("royalty")
//                     .add_attribute("description", "desc1")
//                     .add_attribute("deducted", "10address")
//                     .add_attribute("payment", "recipient2<10address"),
//             ]
//         },
//         res
//     );
// }
