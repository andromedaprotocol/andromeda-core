use andromeda_app::app::{AppComponent, ComponentType};
use andromeda_app_contract::mock::{mock_andromeda_app, MockAppContract};
use andromeda_ibc_registry::mock::{
    mock_andromeda_ibc_registry, mock_ibc_registry_instantiate_msg, MockIbcRegistry,
};
use andromeda_std::{
    amp::AndrAddr,
    os::ibc_registry::{DenomInfo, DenomInfoResponse, IBCDenomInfo},
};
use andromeda_testing::{mock::mock_app, mock_builder::MockAndromedaBuilder, MockContract};
use cosmwasm_std::coin;

#[test]
fn test_ibc_registry() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![coin(100_000, "uandr"), coin(100_000, "uusd")]),
            ("service_address", vec![]),
        ])
        .with_contracts(vec![
            ("app-contract", mock_andromeda_app()),
            ("ibc_registry", mock_andromeda_ibc_registry()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let service_address = andr.get_wallet("service_address");

    let app_code_id = andr.get_code_id(&mut router, "app-contract");

    let ibc_registry_init_msg = mock_ibc_registry_instantiate_msg(
        andr.kernel.addr().clone(),
        None,
        AndrAddr::from_string(service_address),
    );
    let ibc_registry_app_component = AppComponent {
        name: "ibc_registry".to_string(),
        component_type: ComponentType::new(ibc_registry_init_msg),
        ado_type: "ibc_registry".to_string(),
    };

    let app_components = vec![ibc_registry_app_component.clone()];
    let app = MockAppContract::instantiate(
        app_code_id,
        owner,
        &mut router,
        "IBC Registry App",
        app_components,
        andr.kernel.addr(),
        None,
    );

    let ibc_registry: MockIbcRegistry =
        app.query_ado_by_component_name(&router, ibc_registry_app_component.name);

    // Test Store Denom Info

    let ibc_denom_info = IBCDenomInfo {
        denom: "ibc/andr".to_string(),
        denom_info: DenomInfo {
            path: "path".to_string(),
            base_denom: "base_denom".to_string(),
        },
    };
    ibc_registry
        .execute_execute_store_denom_info(
            &mut router,
            service_address.clone(),
            None,
            vec![ibc_denom_info],
        )
        .unwrap();

    let query_res = ibc_registry.query_denom_info(&mut router, "ibc/andr".to_string());
    assert_eq!(
        query_res,
        DenomInfoResponse {
            denom_info: DenomInfo {
                path: "path".to_string(),
                base_denom: "base_denom".to_string(),
            }
        }
    )
}
