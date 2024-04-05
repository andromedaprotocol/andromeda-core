#![cfg(not(target_arch = "wasm32"))]

use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, mock_app_instantiate_msg, MockAppContract};
use andromeda_cw721::mock::{mock_andromeda_cw721, mock_cw721_instantiate_msg};
use andromeda_std::{amp::AndrAddr, os::vfs::convert_component_name};
use andromeda_testing::{mock::mock_app, mock_builder::MockAndromedaBuilder, MockContract};
use cosmwasm_std::{coin, to_json_binary};

#[test]
fn test_app() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![coin(1000, "uandr")]),
            ("user1", vec![]),
        ])
        .with_contracts(vec![
            ("cw721", mock_andromeda_cw721()),
            ("app-contract", mock_andromeda_app()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");

    let app_name = "Simple App";

    // Generate App Components
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        owner.to_string(),
        None,
        andr.kernel.addr().to_string(),
        None,
    );
    let cw721_component = AppComponent::new(
        "cw721".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![cw721_component.clone()];
    let app_init_msg = mock_app_instantiate_msg(
        "SimpleApp".to_string(),
        vec![],
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

    router
        .execute_contract(
            owner.clone(),
            app_addr.clone(),
            &mock_add_app_component_msg(cw721_component),
            &[],
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
            convert_component_name(app_name)
        ))),
    };

    // Create App
    let app_components = vec![cw721_component, cw721_component_with_symlink];
    let app_init_msg = mock_app_instantiate_msg(
        app_name.to_string(),
        app_components.clone(),
        andr.kernel.addr().clone(),
        None,
    );

    let app_code_id = andr.get_code_id(&mut router, "app-contract");
    let app = MockAppContract::instantiate(
        app_code_id,
        owner,
        &mut router,
        "Simple App",
        app_components.clone(),
        andr.kernel.addr(),
        None,
    );

    let components = app.query_components(&router);
    assert_eq!(components, app_components);

    let cw721_component_with_symlink = AppComponent {
        name: "cw721-ref-2".to_string(),
        ado_type: "cw721".to_string(),
        component_type: andromeda_app::app::ComponentType::Symlink(AndrAddr::from_string(format!(
            "~{owner_str}/{0}/cw721",
            convert_component_name(app_init_msg.name.as_str())
        ))),
    };
    app.execute_add_app_component(&mut router, owner.clone(), cw721_component_with_symlink)
        .unwrap();

    let component_addresses = app.query_components(&router);
    assert_eq!(component_addresses.len(), components.len() + 1);

    let cw721_component2 = AppComponent::new(
        "cw721-2".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );
    app.execute_add_app_component(&mut router, owner.clone(), cw721_component2)
        .unwrap();

    let component_addresses = app.query_components(&router);
    assert_eq!(component_addresses.len(), components.len() + 2);
}
