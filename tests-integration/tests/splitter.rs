use andromeda_app::app::{AppComponent, ComponentType};
use andromeda_app_contract::mock::{
    mock_andromeda_app, mock_app_instantiate_msg, mock_get_address_msg,
};

use andromeda_testing::mock::MockAndromeda;

use andromeda_std::amp::Recipient;
use cosmwasm_std::{coin, Addr, Decimal, Uint128};

use andromeda_finance::splitter::{AddressPercent, ExecuteMsg as SplitterExecuteMsg};
use andromeda_splitter::mock::{mock_andromeda_splitter, mock_splitter_instantiate_msg};

use std::str::FromStr;

use cw_multi_test::{App, Executor};
fn mock_app() -> App {
    App::new(|router, _api, storage| {
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked("owner"),
                [coin(10000000, "uandr")].to_vec(),
            )
            .unwrap();
    })
}

fn mock_andromeda(app: &mut App, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

#[test]
fn test_splitter() {
    let owner = Addr::unchecked("owner");
    let recipient_1 = Addr::unchecked("recipient_1");
    let recipient_2 = Addr::unchecked("recipient_2");

    let mut router = mock_app();
    let andr = mock_andromeda(&mut router, owner.clone());

    let app_code_id = router.store_code(mock_andromeda_app());
    andr.store_code_id(&mut router, "app", app_code_id);
    let splitter_code_id = router.store_code(mock_andromeda_splitter());
    andr.store_code_id(&mut router, "splitter", splitter_code_id);

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

    let splitter_init_msg =
        mock_splitter_instantiate_msg(splitter_recipients, andr.kernel_address.clone(), None, None);
    let splitter_app_component = AppComponent {
        name: "1".to_string(),
        component_type: ComponentType::new(splitter_init_msg),
        ado_type: "splitter".to_string(),
    };

    let app_components = vec![splitter_app_component.clone()];

    let app_init_msg = mock_app_instantiate_msg(
        "app".to_string(),
        app_components,
        andr.kernel_address.clone(),
        None,
    );

    let app_addr = router
        .instantiate_contract(
            app_code_id,
            owner.clone(),
            &app_init_msg,
            &[],
            "Splitter App",
            Some(owner.to_string()),
        )
        .unwrap();

    let splitter_addr: String = router
        .wrap()
        .query_wasm_smart(app_addr, &mock_get_address_msg(splitter_app_component.name))
        .unwrap();

    let token = coin(1000, "uandr");
    router
        .execute_contract(
            owner,
            Addr::unchecked(splitter_addr),
            &SplitterExecuteMsg::Send {},
            &[token],
        )
        .unwrap();

    let balance_1 = router.wrap().query_balance(recipient_1, "uandr").unwrap();
    let balance_2 = router.wrap().query_balance(recipient_2, "uandr").unwrap();

    assert_eq!(balance_1.amount, Uint128::from(200u128));
    assert_eq!(balance_2.amount, Uint128::from(800u128));
}
