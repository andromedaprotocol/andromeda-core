use std::str::FromStr;

use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, mock_claim_ownership_msg, MockAppContract};
use andromeda_counter::mock::{mock_andromeda_counter, mock_counter_instantiate_msg, MockCounter};
use andromeda_curve::mock::{mock_andromeda_curve, mock_curve_instantiate_msg, MockCurve};
use andromeda_graph::mock::{mock_andromeda_graph, mock_graph_instantiate_msg, MockGraph};

use andromeda_math::counter::CounterRestriction;
use andromeda_math::curve::{CurveConfig, CurveType};
use andromeda_math::graph::{Coordinate, CoordinateInfo, MapInfo, MapSize, StoredDate};

use andromeda_testing::{
    mock::mock_app, mock_builder::MockAndromedaBuilder, mock_contract::MockContract,
};
use cosmwasm_std::{to_json_binary, Addr, SignedDecimal};
use cw_multi_test::Executor;

#[test]
fn test_curve_app() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![("owner", vec![])])
        .with_contracts(vec![
            ("counter", mock_andromeda_counter()),
            ("curve", mock_andromeda_curve()),
            ("graph", mock_andromeda_graph()),
            ("app-contract", mock_andromeda_app()),
        ])
        .build(&mut router);

    let owner = andr.get_wallet("owner");

    // Generate App Components
    let counter_init_msg = mock_counter_instantiate_msg(
        andr.kernel.addr().to_string(),
        None,
        CounterRestriction::Public,
        Some(1),
        Some(1),
        Some(1),
    );
    let counter_component = AppComponent::new(
        "counter".to_string(),
        "counter".to_string(),
        to_json_binary(&counter_init_msg).unwrap(),
    );

    let curve_init_msg = mock_curve_instantiate_msg(
        andr.kernel.addr().to_string(),
        None,
        CurveConfig::ExpConfig {
            curve_type: CurveType::Growth,
            base_value: 2,
            multiple_variable_value: None,
            constant_value: None,
        },
        None,
    );
    let curve_component = AppComponent::new(
        "curve".to_string(),
        "curve".to_string(),
        to_json_binary(&curve_init_msg).unwrap(),
    );

    let graph_init_msg = mock_graph_instantiate_msg(
        andr.kernel.addr().to_string(),
        None,
        MapInfo {
            map_size: MapSize {
                x_width: 1000,
                y_width: 1000,
                z_width: None,
            },
            allow_negative: false,
            map_decimal: 2,
        },
    );
    let graph_component = AppComponent::new(
        "graph".to_string(),
        "graph".to_string(),
        to_json_binary(&graph_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![
        counter_component.clone(),
        curve_component.clone(),
        graph_component.clone(),
    ];
    let app = MockAppContract::instantiate(
        andr.get_code_id(&mut router, "app-contract"),
        owner,
        &mut router,
        "Curve App",
        app_components,
        andr.kernel.addr(),
        Some(owner.to_string()),
    );

    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(app.addr().clone()),
            &mock_claim_ownership_msg(None),
            &[],
        )
        .unwrap();

    let counter: MockCounter = app.query_ado_by_component_name(&router, counter_component.name);
    let curve: MockCurve = app.query_ado_by_component_name(&router, curve_component.name);
    let graph: MockGraph = app.query_ado_by_component_name(&router, graph_component.name);

    let mut num = 0;
    while num < 5 {
        let x_coordinate = counter.query_current_amount(&mut router).current_amount;
        let y_coordinate = curve.query_plot_y_from_x(&mut router, x_coordinate).y_value;
        graph
            .execute_store_coordinate(
                &mut router,
                owner.clone(),
                Coordinate {
                    x_coordinate: SignedDecimal::from_ratio(x_coordinate, 1),
                    y_coordinate: SignedDecimal::from_str(&y_coordinate).unwrap(),
                    z_coordinate: None,
                },
                false,
                None,
            )
            .unwrap();

        counter
            .execute_increment(&mut router, owner.clone(), None)
            .unwrap();
        num += 1;
    }
    let all_coordinates = graph.query_all_points(&mut router, None, None).points;

    assert_eq!(
        all_coordinates,
        vec![
            (
                CoordinateInfo {
                    x: "1".to_string(),
                    y: "2".to_string(),
                    z: None,
                },
                StoredDate { timestamp: None }
            ),
            (
                CoordinateInfo {
                    x: "2".to_string(),
                    y: "4".to_string(),
                    z: None,
                },
                StoredDate { timestamp: None }
            ),
            (
                CoordinateInfo {
                    x: "3".to_string(),
                    y: "8".to_string(),
                    z: None,
                },
                StoredDate { timestamp: None }
            ),
            (
                CoordinateInfo {
                    x: "4".to_string(),
                    y: "16".to_string(),
                    z: None,
                },
                StoredDate { timestamp: None }
            ),
            (
                CoordinateInfo {
                    x: "5".to_string(),
                    y: "32".to_string(),
                    z: None,
                },
                StoredDate { timestamp: None }
            ),
        ]
    );
}
