use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, MockApp};

use andromeda_testing::{mock::MockAndromeda, mock_contract::MockContract};

use cosmwasm_std::{to_json_binary, Addr};

use cw_multi_test::App;

use andromeda_modules::shunting::{EvaluateParam, EvaluateRefParam, ShuntingResponse};
use andromeda_shunting::mock::{
    mock_andromeda_shunting, mock_shunting_evaluate, mock_shunting_instantiate_msg, MockShunting,
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

    andr.store_ado(&mut router, mock_andromeda_shunting(), "shunting");
    andr.store_ado(&mut router, mock_andromeda_app(), "app");

    // goal: test nested shunting by calculating the area circle
    // user story: want to get the area of the circle using formula `phi * square(r)`
    // phi is passed as param, square(r) should be calculated from a shunting that calculates the square

    // expression for calculating the area of circles. x0 is for phi, x1 is for r squared which is to be calculated by square shunting
    let expressions = vec![
        "{x0}".to_string(),
        "{x1}".to_string(),
        "{x0} * {x1}".to_string(),
    ];

    let shunting_area_msg = mock_shunting_instantiate_msg(expressions, andr.kernel.addr(), None);

    // shunting for calculating circle area
    let shunting_area_component = AppComponent::new(
        "1".to_string(),
        "shunting".to_string(),
        to_json_binary(&shunting_area_msg).unwrap(),
    );

    // expression for square shunting
    let expressions = vec!["{x0}^2".to_string()];
    let shunting_square_msg = mock_shunting_instantiate_msg(expressions, andr.kernel.addr(), None);

    // square shunting component
    let shunting_square_component = AppComponent::new(
        "2".to_string(),
        "shunting".to_string(),
        to_json_binary(&shunting_square_msg).unwrap(),
    );

    let app_components = vec![
        shunting_area_component.clone(),
        shunting_square_component.clone(),
    ];

    let app = MockApp::instantiate(
        andr.get_code_id(&mut router, "app"),
        owner.clone(),
        &mut router,
        "Shunting App",
        app_components,
        andr.kernel.addr(),
        Some(owner.to_string()),
    );

    let square_shunting: MockShunting =
        app.query_ado_by_component_name(&router, shunting_square_component.name);
    let area_shunting: MockShunting =
        app.query_ado_by_component_name(&router, shunting_area_component.name);

    // parameter to be passed for querying circle area shunt. phi is passed as 3.14, r(2) squared is expected to be calculated from square shunting.
    let square_msg = mock_shunting_evaluate(vec![EvaluateParam::Value("2".to_string())]);

    let square_msg_binary = encode_binary(&square_msg);
    let base64_msg = square_msg_binary.expect("converting to base64").to_base64();

    let params = vec![
        EvaluateParam::Value("3.14".to_string()),
        EvaluateParam::Reference(EvaluateRefParam {
            contract: square_shunting.addr().clone(),
            msg: base64_msg,
            accessor: "result".to_string(),
        }),
    ];

    // should return the area of circle whose radius is 2
    let eval_result: ShuntingResponse = area_shunting.evaluate(&router, params);
    assert_eq!(eval_result.result, "12.56".to_string());
}
