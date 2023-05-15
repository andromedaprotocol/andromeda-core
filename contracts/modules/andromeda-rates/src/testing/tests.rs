use crate::contract::{execute, instantiate, query};
use crate::state::CONFIG;
use andromeda_modules::rates::{ExecuteMsg, InstantiateMsg, QueryMsg, RateInfo};
use andromeda_modules::rates::{PaymentsResponse, Rate};
use andromeda_std::ado_base::hooks::OnFundsTransferResponse;
use andromeda_std::ado_base::AndromedaMsg;
use andromeda_std::common::Funds;
use andromeda_std::testing::mock_querier::{mock_dependencies_custom, MOCK_KERNEL_CONTRACT};
use andromeda_std::{
    ado_base::modules::Module,
    amp::{addresses::AndrAddr, recipient::Recipient},
    common::encode_binary,
    error::ContractError,
};
use cosmwasm_std::{
    coin, coins, from_binary,
    testing::{mock_env, mock_info},
    Addr, BankMsg, Coin, CosmosMsg, DepsMut, Response, StdError, SubMsg, Uint128, WasmMsg,
};
use cw_utils::Expiration;
// use andromeda_testing::testing::mock_querier::{
//     mock_dependencies_custom, MOCK_PRIMITIVE_CONTRACT,
// };
// use common::primitive::PrimitivePointer;
// use common::{ado_base::recipient::Recipient, encode_binary};
use cosmwasm_std::testing::mock_dependencies;
use cosmwasm_std::{attr, Decimal, Event};
use cw20::{Cw20Coin, Cw20ExecuteMsg};

const ADDRESS_LIST: &str = "addresslist";
const RATES: &str = "rates";

fn init(deps: DepsMut, modules: Option<Vec<Module>>) -> Response {
    let msg = InstantiateMsg {
        owner: None,
        modules,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        rates: vec![RateInfo {
            rate: todo!(),
            is_additive: todo!(),
            description: todo!(),
            recipients: todo!(),
        }],
    };

    let info = mock_info("owner", &[]);
    instantiate(deps, mock_env(), info, msg).unwrap()
}

#[test]
fn test_instantiate_query() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let owner = "owner";
    let info = mock_info(owner, &[]);
    let rates = vec![
        RateInfo {
            rate: Rate::from(Decimal::percent(10)),
            is_additive: true,
            description: Some("desc1".to_string()),
            recipients: vec![Recipient::new("", None)],
        },
        RateInfo {
            rate: Rate::Flat(Coin {
                amount: Uint128::from(10u128),
                denom: "uusd".to_string(),
            }),
            is_additive: false,
            description: Some("desc2".to_string()),
            recipients: vec![Recipient::new("", None)],
        },
    ];
    let msg = InstantiateMsg {
        rates: rates.clone(),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        modules: None,
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
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let owner = "owner";
    let info = mock_info(owner, &[]);
    let rates = vec![
        RateInfo {
            rate: Rate::from(Decimal::percent(10)),
            is_additive: true,
            description: Some("desc1".to_string()),
            recipients: vec![Recipient::new("", None)],
        },
        RateInfo {
            rate: Rate::Flat(Coin {
                amount: Uint128::from(10u128),
                denom: "uusd".to_string(),
            }),
            is_additive: false,
            description: Some("desc2".to_string()),
            recipients: vec![Recipient::new("", None)],
        },
    ];
    let msg = InstantiateMsg {
        rates: rates.clone(),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        modules: None,
    };
    let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::UpdateRates { rates };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        Response::new().add_attributes(vec![attr("action", "update_rates")]),
        res
    );
}

// #[test]
// fn test_query_deducted_funds_native() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let env = mock_env();
//     let owner = "owner";
//     let info = mock_info(owner, &[]);
//     let rates = vec![
//         RateInfo {
//             rate: Rate::Flat(Coin {
//                 amount: Uint128::from(20u128),
//                 denom: "uusd".to_string(),
//             }),
//             is_additive: true,
//             description: Some("desc2".to_string()),
//             recipients: vec![Recipient::new("1".into(), None)],
//         },
//         RateInfo {
//             rate: Rate::from(Decimal::percent(10)),
//             is_additive: false,
//             description: Some("desc1".to_string()),
//             recipients: vec![Recipient::new("2".into(), None)],
//         },
//         // RateInfo {
//         //     rate: Rate::External(PrimitivePointer {
//         //         address: MOCK_PRIMITIVE_CONTRACT.to_owned(),
//         //         key: Some("flat".into()),
//         //     }),
//         //     is_additive: false,
//         //     description: Some("desc3".to_string()),
//         //     recipients: vec![Recipient::Addr("3".into())],
//         // },
//     ];
//     let msg = InstantiateMsg {
//         rates: rates.clone(),
//         kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
//         owner: None,
//         modules: None,
//     };
//     let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

//     let res: OnFundsTransferResponse = from_binary(
//         &query(
//             deps.as_ref(),
//             env,
//             QueryMsg::AndrQuery(AndromedaQuery::Get(Some(
//                 encode_binary(&Funds::Native(coin(100, "uusd"))).unwrap(),
//             ))),
//         )
//         .unwrap(),
//     )
//     .unwrap();

//     let expected_msgs: Vec<SubMsg> = vec![
//         SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
//             to_address: "1".into(),
//             amount: coins(20, "uusd"),
//         })),
//         SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
//             to_address: "2".into(),
//             amount: coins(10, "uusd"),
//         })),
//         SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
//             to_address: "3".into(),
//             amount: coins(1, "uusd"),
//         })),
//     ];

//     assert_eq!(
//         OnFundsTransferResponse {
//             msgs: expected_msgs,
//             // Deduct 10% from the percent rate, followed by flat fee of 1 from the external rate.
//             leftover_funds: Funds::Native(coin(89, "uusd")),
//             events: vec![
//                 Event::new("tax")
//                     .add_attribute("description", "desc2")
//                     .add_attribute("payment", "1<20uusd"),
//                 Event::new("royalty")
//                     .add_attribute("description", "desc1")
//                     .add_attribute("deducted", "10uusd")
//                     .add_attribute("payment", "2<10uusd"),
//                 Event::new("royalty")
//                     .add_attribute("description", "desc3")
//                     .add_attribute("deducted", "1uusd")
//                     .add_attribute("payment", "3<1uusd"),
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
//             recipients: vec![Recipient::new("1".into(), None)],
//         },
//         RateInfo {
//             rate: Rate::from(Decimal::percent(10)),
//             is_additive: false,
//             description: Some("desc1".to_string()),
//             recipients: vec![Recipient::new("2".into(), None)],
//         },
//         // RateInfo {
//         //     rate: Rate::External(PrimitivePointer {
//         //         address: MOCK_PRIMITIVE_CONTRACT.to_owned(),
//         //         key: Some("flat_cw20".to_string()),
//         //     }),
//         //     is_additive: false,
//         //     description: Some("desc3".to_string()),
//         //     recipients: vec![Recipient::Addr("3".into())],
//         // },
//     ];
//     let msg = InstantiateMsg {
//         rates: rates.clone(),
//         kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
//         owner: None,
//         modules: None,
//     };
//     let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

//     let res: OnFundsTransferResponse = from_binary(
//         &query(
//             deps.as_ref(),
//             env,
//             QueryMsg::AndrQuery(AndromedaQuery::Get(Some(
//                 encode_binary(&Funds::Cw20(Cw20Coin {
//                     amount: 100u128.into(),
//                     address: "address".into(),
//                 }))
//                 .unwrap(),
//             ))),
//         )
//         .unwrap(),
//     )
//     .unwrap();

//     let expected_msgs: Vec<SubMsg> = vec![
//         SubMsg::new(WasmMsg::Execute {
//             contract_addr: cw20_address.to_string(),
//             msg: encode_binary(&Cw20ExecuteMsg::Transfer {
//                 recipient: "1".to_string(),
//                 amount: 20u128.into(),
//             })
//             .unwrap(),
//             funds: vec![],
//         }),
//         SubMsg::new(WasmMsg::Execute {
//             contract_addr: cw20_address.to_string(),
//             msg: encode_binary(&Cw20ExecuteMsg::Transfer {
//                 recipient: "2".to_string(),
//                 amount: 10u128.into(),
//             })
//             .unwrap(),
//             funds: vec![],
//         }),
//         SubMsg::new(WasmMsg::Execute {
//             contract_addr: cw20_address.to_string(),
//             msg: encode_binary(&Cw20ExecuteMsg::Transfer {
//                 recipient: "3".to_string(),
//                 amount: 1u128.into(),
//             })
//             .unwrap(),
//             funds: vec![],
//         }),
//     ];
//     assert_eq!(
//         OnFundsTransferResponse {
//             msgs: expected_msgs,
//             // Deduct 10% from the percent rate, followed by flat fee of 1 from the external rate.
//             leftover_funds: Funds::Cw20(Cw20Coin {
//                 amount: 89u128.into(),
//                 address: cw20_address.to_string()
//             }),
//             events: vec![
//                 Event::new("tax")
//                     .add_attribute("description", "desc2")
//                     .add_attribute("payment", "1<20address"),
//                 Event::new("royalty")
//                     .add_attribute("description", "desc1")
//                     .add_attribute("deducted", "10address")
//                     .add_attribute("payment", "2<10address"),
//                 Event::new("royalty")
//                     .add_attribute("description", "desc3")
//                     .add_attribute("deducted", "1address")
//                     .add_attribute("payment", "3<1address"),
//             ]
//         },
//         res
//     );
// }
