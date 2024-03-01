use andromeda_app::app::{AppComponent, ComponentType};
use andromeda_app_contract::mock::{
    mock_andromeda_app, mock_app_instantiate_msg, mock_get_address_msg,
};

use andromeda_testing::mock::MockAndromeda;

use cosmwasm_std::Addr;

use cw_multi_test::{App, Executor};

use andromeda_modules::shunting::{EvaluateParam, EvaluateRefParam, ShuntingResponse};
use andromeda_shunting::mock::{
    mock_andromeda_shunting, mock_shunting_instantiate_msg, mock_shunting_query_msg,
};
use andromeda_std::common::encode_binary;

fn mock_app() -> App {
    App::new(|router, _api, storage| {
        router
            .bank
            .init_balance(storage, &Addr::unchecked("owner"), vec![])
            .unwrap();
    })
}

fn mock_andromeda(app: &mut App, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

#[test]
fn test_shunting() {
    let owner = Addr::unchecked("owner");

    let mut router = mock_app();
    let andr = mock_andromeda(&mut router, owner.clone());

    let app_code_id = router.store_code(mock_andromeda_app());
    andr.store_code_id(&mut router, "app", app_code_id);

    let shunting_code_id = router.store_code(mock_andromeda_shunting());
    andr.store_code_id(&mut router, "shunting", shunting_code_id);

    // goal: test nested shunting by calculating the area circle
    // user story: want to get the area of the circle using formula `phi * square(r)`
    // phi is passed as param, square(r) should be calculated from a shunting that calculates the square

    // expression for calculating the area of circles. x0 is for phi, x1 is for r squared which is to be calculated by square shunting
    let expressions = vec![
        "{x0}".to_string(),
        "{x1}".to_string(),
        "{x0} * {x1}".to_string(),
    ];

    let shunting_area_msg =
        mock_shunting_instantiate_msg(expressions, andr.kernel_address.clone(), None);

    // shunting for calculating circle area
    let shunting_area_component = AppComponent {
        name: "1".to_string(),
        component_type: ComponentType::new(shunting_area_msg),
        ado_type: "shunting".to_string(),
    };

    // expression for square shunting
    let expressions = vec!["{x0}^2".to_string()];
    let shunting_square_msg =
        mock_shunting_instantiate_msg(expressions, andr.kernel_address.clone(), None);

    // square shunting component
    let shunting_square_component = AppComponent {
        name: "2".to_string(),
        component_type: ComponentType::new(shunting_square_msg),
        ado_type: "shunting".to_string(),
    };

    let app_components = vec![
        shunting_area_component.clone(),
        shunting_square_component.clone(),
    ];

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
            "Shunting App",
            Some(owner.to_string()),
        )
        .unwrap();

    let shunting_square_addr: String = router
        .wrap()
        .query_wasm_smart(
            app_addr.clone(),
            &mock_get_address_msg(shunting_square_component.name),
        )
        .unwrap();

    let shunting_area_addr: String = router
        .wrap()
        .query_wasm_smart(
            app_addr,
            &mock_get_address_msg(shunting_area_component.name),
        )
        .unwrap();

    // parameter to be passed for querying circle area shunt. phi is passed as 3.14, r(2) squared is expected to be calculated from square shunting.
    let square_msg = mock_shunting_query_msg(vec![EvaluateParam::Value("2".to_string())]);

    let square_msg_binary = encode_binary(&square_msg);
    let base64_msg = square_msg_binary.expect("converting to base64").to_base64();

    let params = vec![
        EvaluateParam::Value("3.14".to_string()),
        EvaluateParam::Reference(EvaluateRefParam {
            contract: Addr::unchecked(shunting_square_addr),
            msg: base64_msg,
            accessor: "result".to_string(),
        }),
    ];

    // should return the area of circle whose radius is 2
    let eval_result: ShuntingResponse = router
        .wrap()
        .query_wasm_smart(shunting_area_addr, &mock_shunting_query_msg(params))
        .unwrap();

    assert_eq!(eval_result.result, "12.56".to_string());
}
