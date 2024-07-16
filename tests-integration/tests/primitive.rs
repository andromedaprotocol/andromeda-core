#![cfg(not(target_arch = "wasm32"))]

use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, MockAppContract};
use andromeda_data_storage::primitive::{GetTypeResponse, GetValueResponse, Primitive};

use andromeda_primitive::mock::{
    mock_andromeda_primitive, mock_primitive_instantiate_msg, MockPrimitive,
};
use andromeda_std::{
    ado_base::rates::{LocalRate, LocalRateType, LocalRateValue, PercentRate, Rate},
    amp::Recipient,
    error::ContractError,
};
use andromeda_testing::{mock::mock_app, mock_builder::MockAndromedaBuilder, MockContract};
use cosmwasm_std::{coin, to_json_binary, Decimal, Uint128};

#[test]
fn test_primitive() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![coin(1000, "uandr")]),
            ("buyer_one", vec![coin(1000, "uandr")]),
            ("recipient_one", vec![]),
        ])
        .with_contracts(vec![
            ("app-contract", mock_andromeda_app()),
            ("primitive", mock_andromeda_primitive()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let recipient_one = andr.get_wallet("recipient_one");

    // Generate App Components
    let primitive_init_msg = mock_primitive_instantiate_msg(
        andr.kernel.addr().to_string(),
        None,
        andromeda_data_storage::primitive::PrimitiveRestriction::Private,
    );
    let primitive_component = AppComponent::new(
        "primitive".to_string(),
        "primitive".to_string(),
        to_json_binary(&primitive_init_msg).unwrap(),
    );

    // Create App
    let app_components: Vec<AppComponent> = vec![primitive_component.clone()];
    let app = MockAppContract::instantiate(
        andr.get_code_id(&mut router, "app-contract"),
        owner,
        &mut router,
        "Primitive App",
        app_components,
        andr.kernel.addr(),
        Some(owner.to_string()),
    );

    let primitive: MockPrimitive =
        app.query_ado_by_component_name(&router, primitive_component.name);

    primitive
        .execute_set_value(
            &mut router,
            owner.clone(),
            Some("bool".to_string()),
            Primitive::Bool(true),
            None,
        )
        .unwrap();

    // Check final state
    let get_value_resp: GetValueResponse =
        primitive.query_value(&mut router, Some("bool".to_string()));
    assert_eq!(get_value_resp.value, Primitive::Bool(true));

    let get_type_resp: GetTypeResponse =
        primitive.query_type(&mut router, Some("bool".to_string()));
    assert_eq!(get_type_resp.value_type, "Bool".to_string());

    // Try adding Percentage Rate (should fail)
    let err: ContractError = primitive
        .execute_add_rate(
            &mut router,
            owner.clone(),
            "PrimitiveSetValue".to_string(),
            vec![Rate::Local(LocalRate {
                rate_type: LocalRateType::Deductive,
                recipients: vec![Recipient::new(recipient_one, None)],
                value: LocalRateValue::Percent(PercentRate {
                    percent: Decimal::percent(25),
                }),
                description: None,
            })],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::InvalidRate {});

    // Add flat rate
    primitive
        .execute_add_rate(
            &mut router,
            owner.clone(),
            "PrimitiveSetValue".to_string(),
            vec![Rate::Local(LocalRate {
                rate_type: LocalRateType::Deductive,
                recipients: vec![Recipient::new(recipient_one, None)],
                value: LocalRateValue::Flat(coin(10_u128, "uandr")),
                description: None,
            })],
        )
        .unwrap();

    // Try setting valuw without sending funds
    let err: ContractError = primitive
        .execute_set_value(
            &mut router,
            owner.clone(),
            Some("bool".to_string()),
            Primitive::Bool(true),
            None,
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::InvalidFunds {
            msg: "Zero amounts are prohibited".to_string()
        }
    );

    // Send the exact amount required
    primitive
        .execute_set_value(
            &mut router,
            owner.clone(),
            Some("string".to_string()),
            Primitive::String("StringPrimitive".to_string()),
            Some(coin(10_u128, "uandr".to_string())),
        )
        .unwrap();

    // Check final state
    let get_value_resp: GetValueResponse =
        primitive.query_value(&mut router, Some("string".to_string()));
    assert_eq!(
        get_value_resp.value,
        Primitive::String("StringPrimitive".to_string())
    );

    let get_type_resp: GetTypeResponse =
        primitive.query_type(&mut router, Some("string".to_string()));
    assert_eq!(get_type_resp.value_type, "String".to_string());

    let owner_balance = router.wrap().query_balance(owner, "uandr").unwrap();
    assert_eq!(owner_balance.amount, Uint128::new(990));

    let recipient_balance = router.wrap().query_balance(recipient_one, "uandr").unwrap();
    assert_eq!(recipient_balance.amount, Uint128::new(10));

    // Send more than the required amount to test refunds
    primitive
        .execute_set_value(
            &mut router,
            owner.clone(),
            Some("string".to_string()),
            Primitive::String("StringPrimitive".to_string()),
            Some(coin(200_u128, "uandr".to_string())),
        )
        .unwrap();

    // Check final state
    let get_value_resp: GetValueResponse =
        primitive.query_value(&mut router, Some("string".to_string()));
    assert_eq!(
        get_value_resp.value,
        Primitive::String("StringPrimitive".to_string())
    );

    let get_type_resp: GetTypeResponse =
        primitive.query_type(&mut router, Some("string".to_string()));
    assert_eq!(get_type_resp.value_type, "String".to_string());

    let owner_balance = router.wrap().query_balance(owner, "uandr").unwrap();
    assert_eq!(owner_balance.amount, Uint128::new(980));

    let recipient_balance = router.wrap().query_balance(recipient_one, "uandr").unwrap();
    assert_eq!(recipient_balance.amount, Uint128::new(20));
}
