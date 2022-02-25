use crate::contract::{execute, instantiate};
use andromeda_protocol::{
    communication::modules::{InstantiateType, Module, ModuleType},
    cw20::{ExecuteMsg, InstantiateMsg},
    error::ContractError,
    receipt::{ExecuteMsg as ReceiptExecuteMsg, Receipt},
    testing::mock_querier::{
        mock_dependencies_custom, MOCK_ADDRESSLIST_CONTRACT, MOCK_RATES_CONTRACT,
        MOCK_RECEIPT_CONTRACT,
    },
};
use cosmwasm_std::{
    testing::{mock_env, mock_info},
    to_binary, Addr, CosmosMsg, Event, Response, StdError, SubMsg, Uint128, WasmMsg,
};
use cw20::{Cw20Coin, Cw20ReceiveMsg};
use cw20_base::state::BALANCES;

#[test]
fn test_transfer() {
    // TODO: Test InstantiateType::New() when Fetch contract works.
    let modules: Vec<Module> = vec![
        Module {
            module_type: ModuleType::Receipt,
            instantiate: InstantiateType::Address(MOCK_RECEIPT_CONTRACT.into()),
            is_mutable: false,
        },
        Module {
            module_type: ModuleType::Rates,
            instantiate: InstantiateType::Address(MOCK_RATES_CONTRACT.into()),
            is_mutable: false,
        },
        Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address(MOCK_ADDRESSLIST_CONTRACT.into()),
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
    assert_eq!(Response::default(), res);

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
                events: vec![Event::new("Royalty")],
            },
        })
        .unwrap(),
        funds: vec![],
    }));

    assert_eq!(
        Response::new()
            .add_submessage(receipt_msg)
            .add_event(Event::new("Royalty"))
            .add_attribute("action", "transfer")
            .add_attribute("from", "sender")
            .add_attribute("to", "other")
            .add_attribute("amount", "90"),
        res
    );

    // Funds deducted from the sender.
    assert_eq!(
        Uint128::from(900u128),
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
        Uint128::from(10u128),
        BALANCES
            .load(deps.as_ref().storage, &Addr::unchecked("rates_recipient"))
            .unwrap()
    );
}

#[test]
fn test_send() {
    let modules: Vec<Module> = vec![
        Module {
            module_type: ModuleType::Receipt,
            instantiate: InstantiateType::Address(MOCK_RECEIPT_CONTRACT.into()),
            is_mutable: false,
        },
        Module {
            module_type: ModuleType::Rates,
            instantiate: InstantiateType::Address(MOCK_RATES_CONTRACT.into()),
            is_mutable: false,
        },
        Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address(MOCK_ADDRESSLIST_CONTRACT.into()),
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
    assert_eq!(Response::default(), res);

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
                events: vec![Event::new("Royalty")],
            },
        })
        .unwrap(),
        funds: vec![],
    }));

    assert_eq!(
        Response::new()
            .add_submessage(receipt_msg)
            .add_event(Event::new("Royalty"))
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

    // Funds deducted from the sender.
    assert_eq!(
        Uint128::from(900u128),
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

    // Royalty given to rates_recipient
    assert_eq!(
        Uint128::from(10u128),
        BALANCES
            .load(deps.as_ref().storage, &Addr::unchecked("rates_recipient"))
            .unwrap()
    );
}
