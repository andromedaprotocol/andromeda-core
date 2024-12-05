use andromeda_app::app::{AppComponent, ComponentType};
use andromeda_app_contract::mock::{mock_andromeda_app, MockAppContract};
use andromeda_cw20::mock::{mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_minter, MockCW20};
use andromeda_finance::splitter::{AddressPercent, Cw20HookMsg};
use andromeda_splitter::mock::{
    mock_andromeda_splitter, mock_splitter_instantiate_msg, MockSplitter,
};
use andromeda_std::amp::Recipient;
use andromeda_testing::{mock::mock_app, mock_builder::MockAndromedaBuilder, MockContract};
use cosmwasm_std::{coin, to_json_binary, Decimal, Uint128};
use cw20::Cw20Coin;
use std::str::FromStr;

#[test]
fn test_splitter() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![coin(10000, "uandr")]),
            ("recipient1", vec![]),
            ("recipient2", vec![]),
        ])
        .with_contracts(vec![
            ("app-contract", mock_andromeda_app()),
            ("splitter", mock_andromeda_splitter()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let recipient_1 = andr.get_wallet("recipient1");
    let recipient_2 = andr.get_wallet("recipient2");

    let app_code_id = andr.get_code_id(&mut router, "app-contract");

    let splitter_recipients = vec![
        AddressPercent {
            recipient: Recipient::from_string(recipient_1.to_string()),
            percent: Decimal::from_str("0.2").unwrap(),
        },
        AddressPercent {
            recipient: Recipient::from_string(recipient_2.to_string()),
            percent: Decimal::from_str("0.8").unwrap(),
        },
    ];

    let splitter_init_msg = mock_splitter_instantiate_msg(
        splitter_recipients,
        andr.kernel.addr().clone(),
        None,
        None,
        None,
    );
    let splitter_app_component = AppComponent {
        name: "splitter".to_string(),
        component_type: ComponentType::new(splitter_init_msg),
        ado_type: "splitter".to_string(),
    };

    let app_components = vec![splitter_app_component.clone()];
    let app = MockAppContract::instantiate(
        app_code_id,
        owner,
        &mut router,
        "Splitter App",
        app_components,
        andr.kernel.addr(),
        None,
    );

    let splitter: MockSplitter =
        app.query_ado_by_component_name(&router, splitter_app_component.name);

    let token = coin(1000, "uandr");
    splitter
        .execute_send(&mut router, owner.clone(), &[token.clone()], None)
        .unwrap();

    let balance_1 = router.wrap().query_balance(recipient_1, "uandr").unwrap();
    let balance_2 = router.wrap().query_balance(recipient_2, "uandr").unwrap();

    assert_eq!(balance_1.amount, Uint128::from(200u128));
    assert_eq!(balance_2.amount, Uint128::from(800u128));

    // Test with config
    let custom_recipients = vec![AddressPercent {
        recipient: Recipient::from_string(recipient_1.to_string()),
        percent: Decimal::from_str("0.5").unwrap(),
    }];

    splitter
        .execute_send(
            &mut router,
            owner.clone(),
            &[token],
            Some(custom_recipients),
        )
        .unwrap();
    let balance_1 = router.wrap().query_balance(recipient_1, "uandr").unwrap();
    assert_eq!(balance_1.amount, Uint128::from(200u128 + 500u128));
}

#[test]
fn test_splitter_cw20() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![coin(10000, "uandr")]),
            ("recipient1", vec![]),
            ("recipient2", vec![]),
        ])
        .with_contracts(vec![
            ("app-contract", mock_andromeda_app()),
            ("splitter", mock_andromeda_splitter()),
            ("cw20", mock_andromeda_cw20()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let recipient_1 = andr.get_wallet("recipient1");
    let recipient_2 = andr.get_wallet("recipient2");

    let app_code_id = andr.get_code_id(&mut router, "app-contract");

    let initial_balances = vec![Cw20Coin {
        address: owner.to_string(),
        amount: Uint128::new(1000000u128),
    }];

    let cw20_init_msg = mock_cw20_instantiate_msg(
        None,
        "Test Tokens".to_string(),
        "TTT".to_string(),
        6,
        initial_balances.clone(),
        Some(mock_minter(
            owner.to_string(),
            Some(Uint128::from(10000000000u128)),
        )),
        andr.kernel.addr().to_string(),
    );
    let cw20_component = AppComponent::new(
        "cw20".to_string(),
        "cw20".to_string(),
        to_json_binary(&cw20_init_msg).unwrap(),
    );

    let splitter_recipients = vec![
        AddressPercent {
            recipient: Recipient::from_string(recipient_1.to_string()),
            percent: Decimal::from_str("0.2").unwrap(),
        },
        AddressPercent {
            recipient: Recipient::from_string(recipient_2.to_string()),
            percent: Decimal::from_str("0.8").unwrap(),
        },
    ];

    let splitter_init_msg = mock_splitter_instantiate_msg(
        splitter_recipients,
        andr.kernel.addr().clone(),
        None,
        None,
        None,
    );
    let splitter_app_component = AppComponent {
        name: "splitter".to_string(),
        component_type: ComponentType::new(splitter_init_msg),
        ado_type: "splitter".to_string(),
    };

    let app_components = vec![splitter_app_component.clone(), cw20_component.clone()];
    let app = MockAppContract::instantiate(
        app_code_id,
        owner,
        &mut router,
        "Splitter App",
        app_components,
        andr.kernel.addr(),
        None,
    );

    let splitter: MockSplitter =
        app.query_ado_by_component_name(&router, splitter_app_component.name);

    let cw20: MockCW20 = app.query_ado_by_component_name(&router, cw20_component.name);

    let hook_msg = Cw20HookMsg::Send { config: None };

    cw20.execute_send(
        &mut router,
        owner.clone(),
        splitter.addr(),
        Uint128::new(10),
        &hook_msg,
    )
    .unwrap();

    let cw20_balance = cw20.query_balance(&router, recipient_1);
    assert_eq!(cw20_balance, Uint128::from(2u128));
    let cw20_balance = cw20.query_balance(&router, recipient_2);
    assert_eq!(cw20_balance, Uint128::from(8u128));
}
#[test]
fn test_splitter_cw20_with_remainder() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![coin(10000, "uandr")]),
            ("recipient1", vec![]),
            ("recipient2", vec![]),
            ("recipient3", vec![]),
        ])
        .with_contracts(vec![
            ("app-contract", mock_andromeda_app()),
            ("splitter", mock_andromeda_splitter()),
            ("cw20", mock_andromeda_cw20()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let recipient_1 = andr.get_wallet("recipient1");
    let recipient_2 = andr.get_wallet("recipient2");
    let recipient_3 = andr.get_wallet("recipient3");

    let app_code_id = andr.get_code_id(&mut router, "app-contract");

    let initial_balances = vec![Cw20Coin {
        address: owner.to_string(),
        amount: Uint128::new(1000000u128),
    }];

    let cw20_init_msg = mock_cw20_instantiate_msg(
        None,
        "Test Tokens".to_string(),
        "TTT".to_string(),
        6,
        initial_balances.clone(),
        Some(mock_minter(
            owner.to_string(),
            Some(Uint128::from(10000000000u128)),
        )),
        andr.kernel.addr().to_string(),
    );
    let cw20_component = AppComponent::new(
        "cw20".to_string(),
        "cw20".to_string(),
        to_json_binary(&cw20_init_msg).unwrap(),
    );

    let splitter_recipients = vec![
        AddressPercent {
            recipient: Recipient::from_string(recipient_1.to_string()),
            percent: Decimal::from_str("0.2").unwrap(),
        },
        AddressPercent {
            recipient: Recipient::from_string(recipient_2.to_string()),
            percent: Decimal::from_str("0.3").unwrap(),
        },
    ];

    let splitter_init_msg = mock_splitter_instantiate_msg(
        splitter_recipients,
        andr.kernel.addr().clone(),
        None,
        None,
        Some(Recipient::from_string(recipient_3.to_string())),
    );
    let splitter_app_component = AppComponent {
        name: "splitter".to_string(),
        component_type: ComponentType::new(splitter_init_msg),
        ado_type: "splitter".to_string(),
    };

    let app_components = vec![splitter_app_component.clone(), cw20_component.clone()];
    let app = MockAppContract::instantiate(
        app_code_id,
        owner,
        &mut router,
        "Splitter App",
        app_components,
        andr.kernel.addr(),
        None,
    );

    let splitter: MockSplitter =
        app.query_ado_by_component_name(&router, splitter_app_component.name);

    let cw20: MockCW20 = app.query_ado_by_component_name(&router, cw20_component.name);

    let hook_msg = Cw20HookMsg::Send { config: None };

    cw20.execute_send(
        &mut router,
        owner.clone(),
        splitter.addr(),
        Uint128::new(10),
        &hook_msg,
    )
    .unwrap();

    let cw20_balance = cw20.query_balance(&router, recipient_1);
    assert_eq!(cw20_balance, Uint128::from(2u128));
    let cw20_balance = cw20.query_balance(&router, recipient_2);
    assert_eq!(cw20_balance, Uint128::from(3u128));
    let cw20_balance = cw20.query_balance(&router, recipient_3);
    assert_eq!(cw20_balance, Uint128::from(5u128));
}
