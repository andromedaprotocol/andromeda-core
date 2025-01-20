use andromeda_app::app::{AppComponent, ComponentType};
use andromeda_app_contract::mock::{mock_andromeda_app, MockAppContract};

use andromeda_testing::{mock::mock_app, mock_builder::MockAndromedaBuilder, MockContract};

use andromeda_std::amp::Recipient;
use cosmwasm_std::{coin, Decimal, Uint128};

use andromeda_conditional_splitter::mock::{
    mock_andromeda_conditional_splitter, mock_conditional_splitter_instantiate_msg,
    MockConditionalSplitter,
};
use andromeda_finance::{conditional_splitter::Threshold, splitter::AddressPercent};

use std::str::FromStr;

#[test]
fn test_conditional_splitter() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![coin(100_000, "uandr"), coin(100_000, "uusd")]),
            ("recipient1", vec![]),
            ("recipient2", vec![]),
            ("recipient3", vec![]),
        ])
        .with_contracts(vec![
            ("app-contract", mock_andromeda_app()),
            (
                "conditional-splitter",
                mock_andromeda_conditional_splitter(),
            ),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let recipient_1 = andr.get_wallet("recipient1");
    let recipient_2 = andr.get_wallet("recipient2");
    let recipient_3 = andr.get_wallet("recipient3");

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

    // Percentages that don't add up to 100
    let splitter_recipients3 = vec![
        AddressPercent {
            recipient: Recipient::from_string(recipient_1.to_string()),
            percent: Decimal::from_str("0.2").unwrap(),
        },
        AddressPercent {
            recipient: Recipient::from_string(recipient_2.to_string()),
            percent: Decimal::from_str("0.5").unwrap(),
        },
        AddressPercent {
            recipient: Recipient::from_string(recipient_3.to_string()),
            percent: Decimal::from_str("0.2").unwrap(),
        },
    ];

    let thresholds = vec![
        Threshold::new(Uint128::zero(), splitter_recipients.clone()),
        Threshold::new(Uint128::new(10_000), splitter_recipients),
        Threshold::new(Uint128::new(20_000), splitter_recipients3),
    ];

    let splitter_init_msg = mock_conditional_splitter_instantiate_msg(
        thresholds,
        andr.kernel.addr().clone(),
        None,
        None,
    );
    let splitter_app_component = AppComponent {
        name: "conditional-splitter".to_string(),
        component_type: ComponentType::new(splitter_init_msg),
        ado_type: "conditional-splitter".to_string(),
    };

    let app_components = vec![splitter_app_component.clone()];
    let app = MockAppContract::instantiate(
        app_code_id,
        owner,
        &mut router,
        "Conditional Splitter App",
        app_components,
        andr.kernel.addr(),
        None,
    );

    let splitter: MockConditionalSplitter =
        app.query_ado_by_component_name(&router, splitter_app_component.name);

    let token = coin(1000, "uandr");
    splitter
        .execute_send(&mut router, owner.clone(), &[token])
        .unwrap();

    let balance_1 = router.wrap().query_balance(recipient_1, "uandr").unwrap();
    let balance_2 = router.wrap().query_balance(recipient_2, "uandr").unwrap();

    assert_eq!(balance_1.amount, Uint128::from(200u128));
    assert_eq!(balance_2.amount, Uint128::from(800u128));

    // Second batch
    let token2 = coin(10_000, "uandr");
    splitter
        .execute_send(&mut router, owner.clone(), &[token2])
        .unwrap();

    let balance_1 = router.wrap().query_balance(recipient_1, "uandr").unwrap();
    let balance_2 = router.wrap().query_balance(recipient_2, "uandr").unwrap();

    assert_eq!(balance_1.amount, Uint128::from(200u128 + 2000u128));
    assert_eq!(balance_2.amount, Uint128::from(800u128 + 8000u128));

    // Third batch
    let token2 = coin(50_000, "uandr");
    splitter
        .execute_send(&mut router, owner.clone(), &[token2])
        .unwrap();

    let balance_1 = router.wrap().query_balance(recipient_1, "uandr").unwrap();
    let balance_2 = router.wrap().query_balance(recipient_2, "uandr").unwrap();
    let balance_3 = router.wrap().query_balance(recipient_3, "uandr").unwrap();

    assert_eq!(
        balance_1.amount,
        Uint128::from(200u128 + 2000u128 + 10_000u128)
    );
    assert_eq!(
        balance_2.amount,
        Uint128::from(800u128 + 8000u128 + 25_000u128)
    );
    assert_eq!(balance_3.amount, Uint128::from(10_000u128));

    let balance_owner = router.wrap().query_balance(owner, "uandr").unwrap();
    // First batch was 1000, second batch was 10,000 and both percentages added up to 100, the third batch was 50,000 but the percentages added up to 90, so 45,000 should have been deducted from his balance
    assert_eq!(
        balance_owner.amount,
        Uint128::from(100_000u128 - 1000u128 - 10_000u128 - 45_000u128)
    );

    // Try sending 2 distinct coins
    let uandr_token = coin(10_000, "uandr");
    let uusd_token = coin(100, "uusd");

    splitter
        .execute_send(&mut router, owner.clone(), &[uandr_token, uusd_token])
        .unwrap();

    let uandr_balance_1 = router.wrap().query_balance(recipient_1, "uandr").unwrap();
    let uandr_balance_2 = router.wrap().query_balance(recipient_2, "uandr").unwrap();

    let uusd_balance_1 = router.wrap().query_balance(recipient_1, "uusd").unwrap();
    let uusd_balance_2 = router.wrap().query_balance(recipient_2, "uusd").unwrap();

    assert_eq!(
        uandr_balance_1.amount,
        Uint128::from(200u128 + 2000u128 + 10_000u128 + 2000u128)
    );
    assert_eq!(
        uandr_balance_2.amount,
        Uint128::from(800u128 + 8000u128 + 25_000u128 + 8000u128)
    );

    assert_eq!(uusd_balance_1.amount, Uint128::from(20u128));
    assert_eq!(uusd_balance_2.amount, Uint128::from(80u128));
}

#[test]
fn test_conditional_splitter_with_multiple_thresholds() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![("owner", vec![coin(100_000, "uandr")])])
        .with_contracts(vec![
            ("app-contract", mock_andromeda_app()),
            (
                "conditional-splitter",
                mock_andromeda_conditional_splitter(),
            ),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");

    let recipient_a = "andr12lm0kfn2g3gn39ulzvqnadwksss5ez8rc7rwq7";
    let recipient_b = "andr10dx5rcshf3fwpyw8jjrh5m25kv038xkqvngnls";

    let threshold_recipients_5 = vec![
        AddressPercent {
            recipient: Recipient::from_string(recipient_b.to_string()),
            percent: Decimal::from_str("0.3").unwrap(),
        },
        AddressPercent {
            recipient: Recipient::from_string(recipient_a.to_string()),
            percent: Decimal::from_str("0.7").unwrap(),
        },
    ];

    let threshold_recipients_10 = vec![
        AddressPercent {
            recipient: Recipient::from_string(recipient_a.to_string()),
            percent: Decimal::from_str("0.5").unwrap(),
        },
        AddressPercent {
            recipient: Recipient::from_string(recipient_b.to_string()),
            percent: Decimal::from_str("0.5").unwrap(),
        },
    ];

    let thresholds = vec![
        Threshold::new(Uint128::new(5), threshold_recipients_5),
        Threshold::new(Uint128::new(10), threshold_recipients_10),
    ];

    let app_code_id = andr.get_code_id(&mut router, "app-contract");
    let splitter_init_msg = mock_conditional_splitter_instantiate_msg(
        thresholds,
        andr.kernel.addr().clone(),
        None,
        None,
    );
    let splitter_app_component = AppComponent {
        name: "conditional-splitter".to_string(),
        component_type: ComponentType::new(splitter_init_msg),
        ado_type: "conditional-splitter".to_string(),
    };

    let app_components = vec![splitter_app_component.clone()];
    let app = MockAppContract::instantiate(
        app_code_id,
        owner,
        &mut router,
        "Conditional Splitter App",
        app_components,
        andr.kernel.addr(),
        None,
    );

    let splitter: MockConditionalSplitter =
        app.query_ado_by_component_name(&router, splitter_app_component.name);

    // Test amount between 5 and 10 (should use 30/70 split)
    let token_1 = coin(6, "uandr");
    splitter
        .execute_send(&mut router, owner.clone(), &[token_1])
        .unwrap();

    let balance_a = router.wrap().query_balance(recipient_a, "uandr").unwrap();
    let balance_b = router.wrap().query_balance(recipient_b, "uandr").unwrap();

    assert_eq!(balance_a.amount, Uint128::from(4u128)); // 70% of 6
    assert_eq!(balance_b.amount, Uint128::from(1u128)); // 30% of 6

    // Test amount above 10 (should use 50/50 split)
    let token_2 = coin(15, "uandr");
    splitter
        .execute_send(&mut router, owner.clone(), &[token_2])
        .unwrap();

    let balance_a = router.wrap().query_balance(recipient_a, "uandr").unwrap();
    let balance_b = router.wrap().query_balance(recipient_b, "uandr").unwrap();

    assert_eq!(balance_a.amount, Uint128::from(11u128)); // 4 from previous + 50% of 15
    assert_eq!(balance_b.amount, Uint128::from(8u128)); // 1 from previous + 50% of 15
}
