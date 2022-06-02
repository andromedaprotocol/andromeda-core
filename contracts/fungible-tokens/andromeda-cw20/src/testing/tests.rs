use crate::contract::{execute, instantiate, query};
use andromeda_fungible_tokens::cw20::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_modules::receipt::{ExecuteMsg as ReceiptExecuteMsg, Receipt};
use andromeda_testing::testing::mock_querier::{
    mock_dependencies_custom, MOCK_ADDRESSLIST_CONTRACT, MOCK_RATES_CONTRACT, MOCK_RECEIPT_CONTRACT,
};
use common::{
    ado_base::{
        modules::{Module, ADDRESS_LIST, RATES, RECEIPT},
        AndromedaMsg, AndromedaQuery,
    },
    app::AndrAddress,
    error::ContractError,
};
use cosmwasm_std::{
    testing::{mock_env, mock_info},
    to_binary, Addr, CosmosMsg, Event, Response, StdError, SubMsg, Uint128, WasmMsg,
};
use cw20::{Cw20Coin, Cw20ReceiveMsg};
use cw20_base::state::BALANCES;

#[test]
fn test_andr_query() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("owner", &[]);

    let instantiate_msg = InstantiateMsg {
        name: "Name".into(),
        symbol: "Symbol".into(),
        decimals: 6,
        initial_balances: vec![Cw20Coin {
            amount: 1000u128.into(),
            address: "sender".to_string(),
        }],
        mint: None,
        marketing: None,
        modules: None,
    };

    let _res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

    let msg = QueryMsg::AndrQuery(AndromedaQuery::Owner {});
    let res = query(deps.as_ref(), mock_env(), msg);
    // Test that the query is hooked up correctly.
    assert!(res.is_ok())
}

/*#
 *
 * TODO: Remove when we are happy with InstantiateType replacement.
 * [test]
fn test_instantiate_modules() {
    let receipt_msg = to_binary(&ReceiptInstantiateMsg {
        minter: "minter".to_string(),
        operators: None,
    })
    .unwrap();
    let rates_msg = to_binary(&RatesInstantiateMsg { rates: vec![] }).unwrap();
    let addresslist_msg = to_binary(&AddressListInstantiateMsg {
        operators: vec![],
        is_inclusive: true,
    })
    .unwrap();
    let modules: Vec<Module> = vec![
        Module {
            module_type: RECEIPT.to_owned(),
            instantiate: InstantiateType::New(receipt_msg.clone()),
            is_mutable: false,
        },
        Module {
            module_type: RATES.to_owned(),
            instantiate: InstantiateType::New(rates_msg.clone()),
            is_mutable: false,
        },
        Module {
            module_type: ADDRESS_LIST.to_owned(),
            instantiate: InstantiateType::New(addresslist_msg.clone()),
            is_mutable: false,
        },
    ];
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("sender", &[]);

    let instantiate_msg = InstantiateMsg {
        name: "Name".into(),
        symbol: "Symbol".into(),
        decimals: 6,
        initial_balances: vec![Cw20Coin {
            amount: 1000u128.into(),
            address: "sender".to_string(),
        }],
        mint: None,
        marketing: None,
        modules: Some(modules),
        primitive_contract: MOCK_PRIMITIVE_CONTRACT.to_owned(),
    };

    let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
    assert_eq!(
        "sender",
        ADOContract::default()
            .owner
            .load(deps.as_mut().storage)
            .unwrap()
    );
    assert_eq!(
        "cw20",
        ADOContract::default()
            .ado_type
            .load(deps.as_mut().storage)
            .unwrap()
    );

    let msgs: Vec<SubMsg> = vec![
        SubMsg {
            id: 1,
            reply_on: ReplyOn::Always,
            msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: None,
                code_id: 1,
                msg: receipt_msg,
                funds: vec![],
                label: "Instantiate: receipt".to_string(),
            }),
            gas_limit: None,
        },
        SubMsg {
            id: 2,
            reply_on: ReplyOn::Always,
            msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: None,
                code_id: 2,
                msg: rates_msg,
                funds: vec![],
                label: "Instantiate: rates".to_string(),
            }),
            gas_limit: None,
        },
        SubMsg {
            id: 3,
            reply_on: ReplyOn::Always,
            msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: None,
                code_id: 3,
                msg: addresslist_msg,
                funds: vec![],
                label: "Instantiate: address_list".to_string(),
            }),
            gas_limit: None,
        },
    ];
    assert_eq!(
        Response::new()
            .add_attribute("action", "register_module")
            .add_attribute("action", "register_module")
            .add_attribute("action", "register_module")
            .add_attribute("method", "instantiate")
            .add_attribute("type", "cw20")
            .add_submessages(msgs),
        res
    );
<<<<<<< HEAD
}*/

#[test]
fn test_transfer() {
    let modules: Vec<Module> = vec![
        Module {
            module_type: RECEIPT.to_owned(),
            address: AndrAddress {
                identifier: MOCK_RECEIPT_CONTRACT.to_owned(),
            },
            is_mutable: false,
        },
        Module {
            module_type: RATES.to_owned(),
            address: AndrAddress {
                identifier: MOCK_RATES_CONTRACT.to_owned(),
            },
            is_mutable: false,
        },
        Module {
            module_type: ADDRESS_LIST.to_owned(),
            address: AndrAddress {
                identifier: MOCK_ADDRESSLIST_CONTRACT.to_owned(),
            },
            is_mutable: false,
        },
    ];

    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("sender", &[]);

    let instantiate_msg = InstantiateMsg {
        name: "Name".into(),
        symbol: "Symbol".into(),
        decimals: 6,
        initial_balances: vec![Cw20Coin {
            amount: 1000u128.into(),
            address: "sender".to_string(),
        }],
        mint: None,
        marketing: None,
        modules: Some(modules),
    };

    let res = instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();
    assert_eq!(
        Response::new()
            .add_attribute("action", "register_module")
            .add_attribute("module_idx", "1")
            .add_attribute("action", "register_module")
            .add_attribute("module_idx", "2")
            .add_attribute("action", "register_module")
            .add_attribute("module_idx", "3")
            .add_attribute("method", "instantiate")
            .add_attribute("type", "cw20"),
        res
    );

    assert_eq!(
        Uint128::from(1000u128),
        BALANCES
            .load(deps.as_ref().storage, &Addr::unchecked("sender"))
            .unwrap()
    );

    let msg = ExecuteMsg::Transfer {
        recipient: "other".into(),
        amount: 100u128.into(),
    };

    let not_whitelisted_info = mock_info("not_whitelisted", &[]);
    let res = execute(deps.as_mut(), mock_env(), not_whitelisted_info, msg.clone());
    assert_eq!(
        ContractError::Std(StdError::generic_err(
            "Querier contract error: InvalidAddress"
        )),
        res.unwrap_err()
    );

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let receipt_msg: SubMsg = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: MOCK_RECEIPT_CONTRACT.to_string(),
        msg: to_binary(&ReceiptExecuteMsg::StoreReceipt {
            receipt: Receipt {
                events: vec![Event::new("Royalty"), Event::new("Tax")],
            },
        })
        .unwrap(),
        funds: vec![],
    }));

    assert_eq!(
        Response::new()
            .add_submessage(receipt_msg)
            .add_event(Event::new("Royalty"))
            .add_event(Event::new("Tax"))
            .add_attribute("action", "transfer")
            .add_attribute("from", "sender")
            .add_attribute("to", "other")
            .add_attribute("amount", "90"),
        res
    );

    // Funds deducted from the sender (100 for send, 10 for tax).
    assert_eq!(
        Uint128::from(890u128),
        BALANCES
            .load(deps.as_ref().storage, &Addr::unchecked("sender"))
            .unwrap()
    );

    // Funds given to the receiver.
    assert_eq!(
        Uint128::from(90u128),
        BALANCES
            .load(deps.as_ref().storage, &Addr::unchecked("other"))
            .unwrap()
    );

    // Royalty given to rates_recipient
    assert_eq!(
        Uint128::from(20u128),
        BALANCES
            .load(deps.as_ref().storage, &Addr::unchecked("rates_recipient"))
            .unwrap()
    );
}

#[test]
fn test_send() {
    let modules: Vec<Module> = vec![
        Module {
            module_type: RECEIPT.to_owned(),
            address: AndrAddress {
                identifier: MOCK_RECEIPT_CONTRACT.to_owned(),
            },
            is_mutable: false,
        },
        Module {
            module_type: RATES.to_owned(),
            address: AndrAddress {
                identifier: MOCK_RATES_CONTRACT.to_owned(),
            },
            is_mutable: false,
        },
        Module {
            module_type: ADDRESS_LIST.to_owned(),
            address: AndrAddress {
                identifier: MOCK_ADDRESSLIST_CONTRACT.to_owned(),
            },
            is_mutable: false,
        },
    ];

    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("sender", &[]);

    let instantiate_msg = InstantiateMsg {
        name: "Name".into(),
        symbol: "Symbol".into(),
        decimals: 6,
        initial_balances: vec![Cw20Coin {
            amount: 1000u128.into(),
            address: "sender".to_string(),
        }],
        mint: None,
        marketing: None,
        modules: Some(modules),
    };

    let res = instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();
    assert_eq!(
        Response::new()
            .add_attribute("action", "register_module")
            .add_attribute("module_idx", "1")
            .add_attribute("action", "register_module")
            .add_attribute("module_idx", "2")
            .add_attribute("action", "register_module")
            .add_attribute("module_idx", "3")
            .add_attribute("method", "instantiate")
            .add_attribute("type", "cw20"),
        res
    );

    assert_eq!(
        Uint128::from(1000u128),
        BALANCES
            .load(deps.as_ref().storage, &Addr::unchecked("sender"))
            .unwrap()
    );

    let msg = ExecuteMsg::Send {
        contract: "contract".into(),
        amount: 100u128.into(),
        msg: to_binary(&"msg").unwrap(),
    };

    let not_whitelisted_info = mock_info("not_whitelisted", &[]);
    let res = execute(deps.as_mut(), mock_env(), not_whitelisted_info, msg.clone());
    assert_eq!(
        ContractError::Std(StdError::generic_err(
            "Querier contract error: InvalidAddress"
        )),
        res.unwrap_err()
    );

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let receipt_msg: SubMsg = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: MOCK_RECEIPT_CONTRACT.to_string(),
        msg: to_binary(&ReceiptExecuteMsg::StoreReceipt {
            receipt: Receipt {
                events: vec![Event::new("Royalty"), Event::new("Tax")],
            },
        })
        .unwrap(),
        funds: vec![],
    }));

    assert_eq!(
        Response::new()
            .add_submessage(receipt_msg)
            .add_event(Event::new("Royalty"))
            .add_event(Event::new("Tax"))
            .add_attribute("action", "send")
            .add_attribute("from", "sender")
            .add_attribute("to", "contract")
            .add_attribute("amount", "90")
            .add_message(
                Cw20ReceiveMsg {
                    sender: "sender".into(),
                    amount: 90u128.into(),
                    msg: to_binary(&"msg").unwrap(),
                }
                .into_cosmos_msg("contract")
                .unwrap(),
            ),
        res
    );

    // Funds deducted from the sender (100 for send, 10 for tax).
    assert_eq!(
        Uint128::from(890u128),
        BALANCES
            .load(deps.as_ref().storage, &Addr::unchecked("sender"))
            .unwrap()
    );

    // Funds given to the receiver.
    assert_eq!(
        Uint128::from(90u128),
        BALANCES
            .load(deps.as_ref().storage, &Addr::unchecked("contract"))
            .unwrap()
    );

    // Royalty given to rates_recipient (10 from royalty and 10 from tax)
    assert_eq!(
        Uint128::from(20u128),
        BALANCES
            .load(deps.as_ref().storage, &Addr::unchecked("rates_recipient"))
            .unwrap()
    );
}

#[test]
fn test_update_app_contract() {
    let mut deps = mock_dependencies_custom(&[]);

    let modules: Vec<Module> = vec![Module {
        module_type: ADDRESS_LIST.to_owned(),
        address: AndrAddress {
            identifier: MOCK_ADDRESSLIST_CONTRACT.to_owned(),
        },
        is_mutable: false,
    }];

    let info = mock_info("app_contract", &[]);
    let instantiate_msg = InstantiateMsg {
        name: "Name".into(),
        symbol: "Symbol".into(),
        decimals: 6,
        initial_balances: vec![Cw20Coin {
            amount: 1000u128.into(),
            address: "sender".to_string(),
        }],
        mint: None,
        marketing: None,
        modules: Some(modules),
    };

    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::UpdateAppContract {
        address: "app_contract".to_string(),
    });

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "update_app_contract")
            .add_attribute("address", "app_contract"),
        res
    );
}
