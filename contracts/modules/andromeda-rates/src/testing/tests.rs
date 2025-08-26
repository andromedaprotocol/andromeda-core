use crate::contract::{execute, instantiate, query, query_deducted_funds};
use crate::testing::mock_querier::{mock_dependencies_custom, MOCK_KERNEL_CONTRACT, MOCK_OWNER};
use andromeda_modules::rates::{ExecuteMsg, InstantiateMsg, QueryMsg, RateResponse};
use andromeda_std::{
    ado_base::rates::{LocalRate, LocalRateType, LocalRateValue, RatesResponse},
    amp::{recipient::Recipient, AndrAddr},
    common::{encode_binary, Funds},
    testing::mock_querier::MOCK_CW20_CONTRACT,
};
use cosmwasm_std::Addr;
use cosmwasm_std::{
    attr, coin, coins,
    testing::{message_info, mock_env},
    BankMsg, CosmosMsg, Event, Response, SubMsg, WasmMsg,
};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
const RECIPIENT: &str = "cosmwasm1vewsdxxmeraett7ztsaym88jsrv85kzm0xvjg09xqz8aqvjcja0syapxq9";

#[test]
fn test_instantiate_query() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let owner = deps.api.addr_make("owner");
    let mock_uandr = Addr::unchecked(MOCK_CW20_CONTRACT);
    let info = message_info(&owner, &[]);
    let action = "deposit".to_string();
    let rate = LocalRate {
        rate_type: LocalRateType::Additive,
        recipient: Recipient {
            address: AndrAddr::from_string(owner.to_string()),
            msg: None,
            ibc_recovery_address: None,
        },
        value: LocalRateValue::Flat(coin(100_u128, mock_uandr.to_string())),
        description: None,
    };
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
    let owner = deps.api.addr_make("owner");
    let info = message_info(&owner, &[]);
    let action: String = "deposit".to_string();
    let rate = LocalRate {
        rate_type: LocalRateType::Additive,
        recipient: Recipient {
            address: AndrAddr::from_string("owner".to_string()),
            msg: None,
            ibc_recovery_address: None,
        },
        value: LocalRateValue::Flat(coin(100_u128, MOCK_CW20_CONTRACT)),
        description: None,
    };
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        action: action.clone(),
        rate: rate.clone(),
    };
    let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    // Update rate
    let msg = ExecuteMsg::SetRate { action, rate };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    let expected_res: Response = Response::new().add_attribute("action", "set_rate");
    for attr in expected_res.attributes {
        assert!(
            res.attributes.contains(&attr),
            "Attribute {:?} not found",
            attr,
        );
    }
    for msg in expected_res.messages {
        assert!(res.messages.contains(&msg), "Message {:?} not found", msg,);
    }
    for event in expected_res.events {
        assert!(res.events.contains(&event), "Event {:?} not found", event,);
    }
}

#[test]
fn test_query_deducted_funds_native() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let mock_owner = deps.api.addr_make(MOCK_OWNER);
    let info = message_info(&mock_owner, &[coin(1000, "uusd")]);
    let action: String = "deposit".to_string();
    let payload = encode_binary(&action).unwrap();
    let rate = LocalRate {
        rate_type: LocalRateType::Additive,
        recipient: Recipient {
            address: AndrAddr::from_string(RECIPIENT.to_string()),
            msg: None,
            ibc_recovery_address: None,
        },
        value: LocalRateValue::Flat(coin(20_u128, MOCK_CW20_CONTRACT)),
        description: None,
    };
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        action,
        rate,
    };
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    let res = query_deducted_funds(
        deps.as_ref(),
        payload,
        Funds::Native(coin(100, MOCK_CW20_CONTRACT)),
    )
    .unwrap();

    let expected_msgs: Vec<SubMsg> = vec![
        SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: RECIPIENT.into(),
            amount: coins(20, MOCK_CW20_CONTRACT),
        })),
        // SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        //     to_address: MOCK_RECIPIENT2.into(),
        //     amount: coins(10, "uusd"),
        // })),
    ];

    assert_eq!(
        RatesResponse {
            msgs: expected_msgs,
            leftover_funds: Funds::Native(coin(100, MOCK_CW20_CONTRACT)),
            events: vec![
                Event::new("tax")
                    .add_attribute("payment", format!("{}<20{}", RECIPIENT, MOCK_CW20_CONTRACT),),
                // Event::new("royalty")
                //     .add_attribute("description", "desc1")
                //     .add_attribute("deducted", "10uusd")
                //     .add_attribute("payment", "recipient2<10uusd"),
            ]
        },
        res
    );
}

#[test]
fn test_query_deducted_funds_cw20() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let owner = deps.api.addr_make("owner");
    let info = message_info(&owner, &[]);

    let action: String = "deposit".to_string();
    let payload = encode_binary(&action).unwrap();
    let recipient1 = deps.api.addr_make(RECIPIENT);
    let rate = LocalRate {
        rate_type: LocalRateType::Additive,
        recipient: Recipient {
            address: AndrAddr::from_string(recipient1.to_string()),
            msg: None,
            ibc_recovery_address: None,
        },
        value: LocalRateValue::Flat(coin(20_u128, MOCK_CW20_CONTRACT)),
        description: None,
    };

    // let rates = vec![
    //     RateInfo {
    //         rate: Rate::Flat(Coin {
    //             amount: Uint128::from(20u128),
    //             denom: cw20_address.to_string(),
    //         }),
    //         is_additive: true,
    //         description: Some("desc2".to_string()),
    //         recipients: vec![Recipient::new(MOCK_RECIPIENT1, None)],
    //     },
    //     RateInfo {
    //         rate: Rate::from(Decimal::percent(10)),
    //         is_additive: false,
    //         description: Some("desc1".to_string()),
    //         recipients: vec![Recipient::new(MOCK_RECIPIENT2, None)],
    //     },
    // ];
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        action,
        rate,
    };
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let res: RatesResponse = query_deducted_funds(
        deps.as_ref(),
        payload,
        Funds::Cw20(Cw20Coin {
            amount: 100u128.into(),
            address: MOCK_CW20_CONTRACT.to_string(),
        }),
    )
    .unwrap();

    let expected_msgs: Vec<SubMsg> = vec![
        SubMsg::new(WasmMsg::Execute {
            contract_addr: MOCK_CW20_CONTRACT.to_string(),
            msg: encode_binary(&Cw20ExecuteMsg::Transfer {
                recipient: recipient1.to_string(),
                amount: 20u128.into(),
            })
            .unwrap(),
            funds: vec![],
        }),
        // SubMsg::new(WasmMsg::Execute {
        //     contract_addr: cw20_address.to_string(),
        //     msg: encode_binary(&Cw20ExecuteMsg::Transfer {
        //         recipient: MOCK_RECIPIENT2.to_string(),
        //         amount: 10u128.into(),
        //     })
        //     .unwrap(),
        //     funds: vec![],
        // }),
    ];
    assert_eq!(
        RatesResponse {
            msgs: expected_msgs,
            leftover_funds: Funds::Cw20(Cw20Coin {
                amount: 100u128.into(),
                address: MOCK_CW20_CONTRACT.to_string()
            }),
            events: vec![Event::new("tax").add_attribute(
                "payment",
                format!("{}<20{}", recipient1, MOCK_CW20_CONTRACT)
            ),]
        },
        res
    );
}
