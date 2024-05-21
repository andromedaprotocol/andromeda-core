#![cfg(not(target_arch = "wasm32"))]

use andromeda_data_storage::primitive::{GetTypeResponse, GetValueResponse, Primitive};

use andromeda_primitive::mock::{
    mock_andromeda_primitive, mock_primitive_get_type, mock_primitive_get_value,
    mock_primitive_instantiate_msg, mock_store_value_msg,
};
use andromeda_testing::{mock::mock_app, mock_builder::MockAndromedaBuilder, MockContract};
use cw_multi_test::Executor;

#[test]
fn test_primtive() {
    let mut router = mock_app(None);

    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![("owner", vec![])])
        .with_contracts(vec![("primitive", mock_andromeda_primitive())])
        .build(&mut router);
    let sender = andr.get_wallet("owner");
    // Store contract codes
    let primtive_code_id = router.store_code(mock_andromeda_primitive());

    andr.store_code_id(&mut router, "primitve", primtive_code_id);

    let primitive_init_msg = mock_primitive_instantiate_msg(
        andr.kernel.addr().to_string(),
        None,
        andromeda_data_storage::primitive::PrimitiveRestriction::Private,
    );

    let primitive_addr = router
        .instantiate_contract(
            primtive_code_id,
            sender.clone(),
            &primitive_init_msg,
            &[],
            "Auction App",
            Some(sender.to_string()),
        )
        .unwrap();

    // Claim Ownership
    router
        .execute_contract(
            sender.clone(),
            primitive_addr.clone(),
            &mock_store_value_msg(Some("key".to_string()), Primitive::Bool(true)),
            &[],
        )
        .unwrap();

    // Check final state
    let get_value_resp: GetValueResponse = router
        .wrap()
        .query_wasm_smart(
            primitive_addr.clone(),
            &mock_primitive_get_value(Some("key".to_string())),
        )
        .unwrap();
    assert_eq!(get_value_resp.value, Primitive::Bool(true));

    let get_type_resp: GetTypeResponse = router
        .wrap()
        .query_wasm_smart(
            primitive_addr,
            &mock_primitive_get_type(Some("key".to_string())),
        )
        .unwrap();
    assert_eq!(get_type_resp.value_type, "Bool".to_string());
}

// #![cfg(not(target_arch = "wasm32"))]

// use andromeda_app::app::AppComponent;
// use andromeda_app_contract::mock::{mock_claim_ownership_msg, MockAppContract};
// use andromeda_data_storage::primitive::{GetValueResponse, Primitive};

// use andromeda_primitive::mock::{
//     mock_andromeda_primitive, mock_primitive_get_value, mock_primitive_instantiate_msg,
//     mock_store_value_msg, MockPrimitive,
// };
// use andromeda_testing::{mock::mock_app, mock_builder::MockAndromedaBuilder, MockContract};
// use cosmwasm_schema::schemars::Map;
// use cosmwasm_std::{coin, to_json_binary, Addr};
// use cw_multi_test::Executor;

// #[test]
// fn test_primtive() {
//     let mut router = mock_app(None);
//     let andr = MockAndromedaBuilder::new(&mut router, "admin")
//         .with_wallets(vec![
//             ("owner", vec![]),
//             ("buyer_one", vec![coin(1000, "uandr")]),
//             ("recipient_one", vec![]),
//         ])
//         .with_contracts(vec![("primitive", mock_andromeda_primitive())])
//         .build(&mut router);
//     let owner = andr.get_wallet("owner");

//     // Generate App Components
//     let primitive_init_msg = mock_primitive_instantiate_msg(
//         andr.kernel.addr().to_string(),
//         None,
//         andromeda_data_storage::primitive::PrimitiveRestriction::Private,
//     );
//     let primitive_component = AppComponent::new(
//         "primitive".to_string(),
//         "primitive".to_string(),
//         to_json_binary(&primitive_init_msg).unwrap(),
//     );

//     // Create App
//     let app_components: Vec<AppComponent> = vec![primitive_component.clone()];
//     let app = MockAppContract::instantiate(
//         andr.get_code_id(&mut router, "app-contract"),
//         owner,
//         &mut router,
//         "Primitive App",
//         app_components,
//         andr.kernel.addr(),
//         Some(owner.to_string()),
//     );

//     // router
//     //     .execute_contract(
//     //         owner.clone(),
//     //         Addr::unchecked(app.addr().clone()),
//     //         &mock_claim_ownership_msg(None),
//     //         &[],
//     //     )
//     //     .unwrap();

//     // let primitive: MockPrimitive =
//     //     app.query_ado_by_component_name(&router, primitive_component.name);

//     // primitive
//     //     .execute_set_value(
//     //         &mut router,
//     //         owner.clone(),
//     //         Some("bool".to_string()),
//     //         Primitive::Bool(true),
//     //     )
//     //     .unwrap();

//     // // Check final state
//     // let get_value_resp: GetValueResponse = router
//     //     .wrap()
//     //     .query_wasm_smart(
//     //         primitive.addr(),
//     //         &mock_primitive_get_value(Some("bool".to_string())),
//     //     )
//     //     .unwrap();
//     // assert_eq!(get_value_resp.value, Primitive::Bool(true));
// }
