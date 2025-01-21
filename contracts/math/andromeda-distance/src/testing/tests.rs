use std::str::FromStr;

use super::mock::{proper_initialization, query_distance, query_manhattan_distance};
use andromeda_math::distance::Coordinate;
use andromeda_std::error::ContractError;
use cosmwasm_std::SignedDecimal;

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
            x_coordinate: SignedDecimal::one(),
            y_coordinate: SignedDecimal::one(),
            z_coordinate: None,
        },
        Coordinate {
            x_coordinate: SignedDecimal::zero(),
            y_coordinate: SignedDecimal::zero(),
            z_coordinate: None,
        },
        5,
    )
    .unwrap();
    assert_eq!(query_res, "1.4142135623730951".to_string());

    let query_res = query_manhattan_distance(
        deps.as_ref(),
        Coordinate {
            x_coordinate: SignedDecimal::one(),
            y_coordinate: SignedDecimal::one(),
            z_coordinate: None,
        },
        Coordinate {
            x_coordinate: SignedDecimal::zero(),
            y_coordinate: SignedDecimal::zero(),
            z_coordinate: None,
        },
        5,
    )
    .unwrap();
    assert_eq!(query_res, "2".to_string());

    let query_res = query_distance(
        deps.as_ref(),
        Coordinate {
            x_coordinate: SignedDecimal::one(),
            y_coordinate: SignedDecimal::one(),
            z_coordinate: Some(SignedDecimal::one()),
        },
        Coordinate {
            x_coordinate: SignedDecimal::zero(),
            y_coordinate: SignedDecimal::zero(),
            z_coordinate: Some(SignedDecimal::zero()),
        },
        5,
    )
    .unwrap();
    assert_eq!(query_res, "1.7320508075688772".to_string());

    let query_res = query_manhattan_distance(
        deps.as_ref(),
        Coordinate {
            x_coordinate: SignedDecimal::one(),
            y_coordinate: SignedDecimal::one(),
            z_coordinate: Some(SignedDecimal::one()),
        },
        Coordinate {
            x_coordinate: SignedDecimal::zero(),
            y_coordinate: SignedDecimal::zero(),
            z_coordinate: Some(SignedDecimal::zero()),
        },
        5,
    )
    .unwrap();
    assert_eq!(query_res, "3".to_string());

    let query_res = query_distance(
        deps.as_ref(),
        Coordinate {
            x_coordinate: SignedDecimal::from_str("10").unwrap(),
            y_coordinate: SignedDecimal::from_str("10").unwrap(),
            z_coordinate: None,
        },
        Coordinate {
            x_coordinate: SignedDecimal::from_str("-10").unwrap(),
            y_coordinate: SignedDecimal::from_str("-10").unwrap(),
            z_coordinate: None,
        },
        5,
    )
    .unwrap();
    assert_eq!(query_res, "28.284271247461902".to_string());

    let err_res = query_distance(
        deps.as_ref(),
        Coordinate {
            x_coordinate: SignedDecimal::from_str("10").unwrap(),
            y_coordinate: SignedDecimal::from_str("10").unwrap(),
            z_coordinate: None,
        },
        Coordinate {
            x_coordinate: SignedDecimal::from_str("-10").unwrap(),
            y_coordinate: SignedDecimal::from_str("-10").unwrap(),
            z_coordinate: None,
        },
        25,
    )
    .unwrap_err();

    assert_eq!(
        err_res,
        ContractError::InvalidParameter {
            error: Some("Decimal value too large".to_string())
        },
    );
}
