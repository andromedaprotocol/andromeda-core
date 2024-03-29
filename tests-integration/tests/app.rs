#![cfg(not(target_arch = "wasm32"))]

use andromeda_app::app::{AppComponent, ComponentAddress};
use andromeda_app_contract::mock::{
    mock_add_app_component_msg, mock_andromeda_app, mock_app_instantiate_msg,
    mock_get_adresses_with_names_msg, mock_get_components_msg,
};
use andromeda_cw721::mock::{mock_andromeda_cw721, mock_cw721_instantiate_msg};
use andromeda_std::amp::AndrAddr;
use andromeda_testing::mock::MockAndromeda;
use cosmwasm_std::{coin, to_json_binary, Addr};
use cw_multi_test::{
    App, AppBuilder, BankKeeper, Executor, MockAddressGenerator, MockApiBech32, WasmKeeper,
};

fn mock_app() -> App<BankKeeper, MockApiBech32> {
    AppBuilder::new()
        .with_api(MockApiBech32::new("andr"))
        .with_wasm(WasmKeeper::new().with_address_generator(MockAddressGenerator))
        .build(|router, _api, storage| {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked("owner"),
                    [coin(9999999, "uandr")].to_vec(),
                )
                .unwrap();
        })
}

fn mock_andromeda(app: &mut App<BankKeeper, MockApiBech32>, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

#[test]
fn test_app() {
    let mut router = mock_app();
    let owner = router.api().addr_make("owner");

    let andr = mock_andromeda(&mut router, owner.clone());

    // Store contract codes
    let cw721_code_id = router.store_code(mock_andromeda_cw721());
    let app_code_id = router.store_code(mock_andromeda_app());
    andr.store_code_id(&mut router, "cw721", cw721_code_id);
    andr.store_code_id(&mut router, "app-contract", app_code_id);

    // Generate App Components
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        owner.to_string(),
        None,
        andr.kernel_address.to_string(),
        None,
    );
    let cw721_component = AppComponent::new(
        "cw721".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![cw721_component];
    let app_init_msg = mock_app_instantiate_msg(
        "SimpleApp".to_string(),
        app_components.clone(),
        andr.kernel_address.clone(),
        None,
    );

    let app_addr = router
        .instantiate_contract(
            app_code_id,
            owner.clone(),
            &app_init_msg,
            &[],
            "Simple App",
            Some(owner.to_string()),
        )
        .unwrap();

    let components: Vec<AppComponent> = router
        .wrap()
        .query_wasm_smart(app_addr.clone(), &mock_get_components_msg())
        .unwrap();
    assert_eq!(components, app_components);

    let component_addresses: Vec<ComponentAddress> = router
        .wrap()
        .query_wasm_smart(app_addr.clone(), &mock_get_adresses_with_names_msg())
        .unwrap();
    assert_eq!(component_addresses.len(), components.len());

    let owner_str = owner.to_string();
    let cw721_component_with_symlink = AppComponent {
        name: "cw721-ref".to_string(),
        ado_type: "cw721".to_string(),
        component_type: andromeda_app::app::ComponentType::Symlink(AndrAddr::from_string(format!(
            "~{owner_str}/{0}/cw721",
            app_init_msg.name
        ))),
    };
    router
        .execute_contract(
            owner,
            app_addr.clone(),
            &mock_add_app_component_msg(cw721_component_with_symlink),
            &[],
        )
        .unwrap();

    let component_addresses: Vec<ComponentAddress> = router
        .wrap()
        .query_wasm_smart(app_addr, &mock_get_adresses_with_names_msg())
        .unwrap();
    assert_eq!(component_addresses.len(), components.len() + 1);
}
