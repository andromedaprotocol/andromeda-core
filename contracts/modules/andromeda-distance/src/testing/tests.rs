use super::mock::{proper_initialization, query_distance};
use andromeda_modules::distance::Coordinate;

#[test]
fn test_instantiation() {
    proper_initialization();
}

#[test]
fn test_query_distance() {
    let (deps, _) = proper_initialization();

    let query_res = query_distance(
        deps.as_ref(),
        Coordinate {
            x_coordinate: 1_f64,
            y_coordinate: 1_f64,
            z_coordinate: None,
        },
        Coordinate {
            x_coordinate: 0_f64,
            y_coordinate: 0_f64,
            z_coordinate: None,
        },
        5,
    )
    .unwrap();
    assert_eq!(query_res, "1.41421".to_string(),);

    let query_res = query_distance(
        deps.as_ref(),
        Coordinate {
            x_coordinate: 1_f64,
            y_coordinate: 1_f64,
            z_coordinate: Some(1_f64),
        },
        Coordinate {
            x_coordinate: 0_f64,
            y_coordinate: 0_f64,
            z_coordinate: Some(0_f64),
        },
        5,
    )
    .unwrap();
    assert_eq!(query_res, "1.73205".to_string(),);

    let query_res = query_distance(
        deps.as_ref(),
        Coordinate {
            x_coordinate: 10_f64,
            y_coordinate: 10_f64,
            z_coordinate: None,
        },
        Coordinate {
            x_coordinate: -10_f64,
            y_coordinate: -10_f64,
            z_coordinate: None,
        },
        5,
    )
    .unwrap();
    assert_eq!(query_res, "28.28427".to_string(),);
}
