use andromeda_app::app::{AppComponent, ComponentType};
use andromeda_app_contract::mock::{mock_andromeda_app, MockAppContract};

use andromeda_testing::{mock::mock_app, mock_builder::MockAndromedaBuilder, MockContract};

use andromeda_std::amp::Recipient;
use cosmwasm_std::{coin, coins, Uint128};

use andromeda_finance::set_amount_splitter::AddressAmount;
use andromeda_set_amount_splitter::mock::{
    mock_andromeda_set_amount_splitter, mock_set_amount_splitter_instantiate_msg,
    MockSetAmountSplitter,
};

#[test]
fn test_splitter() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![coin(1000, "uandr")]),
            ("recipient1", vec![]),
            ("recipient2", vec![]),
        ])
        .with_contracts(vec![
            ("app-contract", mock_andromeda_app()),
            ("splitter", mock_andromeda_set_amount_splitter()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let recipient_1 = andr.get_wallet("recipient1");
    let recipient_2 = andr.get_wallet("recipient2");

    let app_code_id = andr.get_code_id(&mut router, "app-contract");

    let splitter_recipients = vec![
        AddressAmount {
            recipient: Recipient::from_string(recipient_1.to_string()),
            coins: coins(100_u128, "uandr"),
        },
        AddressAmount {
            recipient: Recipient::from_string(recipient_2.to_string()),
            coins: coins(50_u128, "uandr"),
        },
    ];

    let splitter_init_msg = mock_set_amount_splitter_instantiate_msg(
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

    let splitter: MockSetAmountSplitter =
        app.query_ado_by_component_name(&router, splitter_app_component.name);

    let token = coin(1000, "uandr");
    splitter
        .execute_send(&mut router, owner.clone(), &[token], None)
        .unwrap();

    let balance_1 = router.wrap().query_balance(recipient_1, "uandr").unwrap();
    let balance_2 = router.wrap().query_balance(recipient_2, "uandr").unwrap();
    let balance_owner = router.wrap().query_balance(owner, "uandr").unwrap();

    assert_eq!(balance_1.amount, Uint128::from(100u128));
    assert_eq!(balance_2.amount, Uint128::from(50u128));
    // The owner sent 1000 but only 150 was needed. His account should be now worth 850
    assert_eq!(balance_owner.amount, Uint128::from(850u128));
}
