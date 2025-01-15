use crate::contract::query;
use andromeda_math::point::{GetDataOwnerResponse, PointCoordinate, PointRestriction, QueryMsg};
use cosmwasm_std::{from_json, testing::mock_env};

use andromeda_std::{amp::AndrAddr, error::ContractError};

use super::mock::{delete_point, proper_initialization, query_point, set_point};

#[test]
fn test_instantiation() {
    proper_initialization(PointRestriction::Private);
}

#[test]
fn test_set_and_update_point() {
    let (mut deps, info) = proper_initialization(PointRestriction::Private);
    let point = PointCoordinate::from_f64(10_f64, 10_f64, Some(10_f64));
    point.validate().unwrap();
    set_point(deps.as_mut(), &point, info.sender.as_ref()).unwrap();

    let query_res: PointCoordinate = query_point(deps.as_ref()).unwrap();

    assert_eq!(point, query_res);

    let point = PointCoordinate::from_f64(5_f64, 5_f64, Some(5_f64));
    point.validate().unwrap();
    set_point(deps.as_mut(), &point, info.sender.as_ref()).unwrap();

    let query_res: PointCoordinate = query_point(deps.as_ref()).unwrap();

    assert_eq!(point, query_res);
}

struct TestHandlePointCoordinate {
    name: &'static str,
    point_coordinate: PointCoordinate,
    expected_error: Option<ContractError>,
}

#[test]
fn test_set_point_invalid() {
    let test_cases = vec![
        TestHandlePointCoordinate {
            name: "Invalid x_coordinate",
            point_coordinate: PointCoordinate {
                x_coordinate: "10.abc".to_string(),
                y_coordinate: "10".to_string(),
                z_coordinate: Some("10".to_string()),
            },
            expected_error: Some(ContractError::ParsingError {
                err: "x_coordinate: can not parse to f64".to_string(),
            }),
        },
        TestHandlePointCoordinate {
            name: "Invalid y_coordinate",
            point_coordinate: PointCoordinate {
                x_coordinate: "10".to_string(),
                y_coordinate: "10.abc".to_string(),
                z_coordinate: None,
            },
            expected_error: Some(ContractError::ParsingError {
                err: "y_coordinate: can not parse to f64".to_string(),
            }),
        },
        TestHandlePointCoordinate {
            name: "Invalid z_coordinate",
            point_coordinate: PointCoordinate {
                x_coordinate: "10".to_string(),
                y_coordinate: "10".to_string(),
                z_coordinate: Some("10.abc".to_string()),
            },
            expected_error: Some(ContractError::ParsingError {
                err: "z_coordinate: can not parse to f64".to_string(),
            }),
        },
    ];

    for test in test_cases {
        let res = test.point_coordinate.validate();

        if let Some(err) = test.expected_error {
            assert_eq!(res.unwrap_err(), err, "{}", test.name);
            continue;
        }

        assert!(res.is_ok())
    }
}

#[test]
fn test_delete_point() {
    let (mut deps, info) = proper_initialization(PointRestriction::Private);
    let point = PointCoordinate::from_f64(10_f64, 10_f64, Some(10_f64));
    set_point(deps.as_mut(), &point, info.sender.as_ref()).unwrap();
    delete_point(deps.as_mut(), info.sender.as_ref()).unwrap();
    query_point(deps.as_ref()).unwrap_err();
}

#[test]
fn test_restriction_private() {
    let (mut deps, info) = proper_initialization(PointRestriction::Private);

    let point = PointCoordinate::from_f64(10_f64, 10_f64, Some(10_f64));
    let external_user = "external".to_string();

    // Set Point as owner
    set_point(deps.as_mut(), &point, info.sender.as_ref()).unwrap();
    delete_point(deps.as_mut(), info.sender.as_ref()).unwrap();
    query_point(deps.as_ref()).unwrap_err();

    // Set Point as external user
    // This should error
    set_point(deps.as_mut(), &point, &external_user).unwrap_err();
    // Set a point by owner so we can test delete for it
    set_point(deps.as_mut(), &point, info.sender.as_ref()).unwrap();
    // Delete point set by owner by external user
    // This will error
    delete_point(deps.as_mut(), &external_user).unwrap_err();

    // Point is still present
    query_point(deps.as_ref()).unwrap();
}

#[test]
fn test_restriction_public() {
    let (mut deps, info) = proper_initialization(PointRestriction::Public);

    let point = PointCoordinate::from_f64(10_f64, 10_f64, Some(10_f64));
    let external_user = "external".to_string();

    // Set Point as owner
    set_point(deps.as_mut(), &point, info.sender.as_ref()).unwrap();
    delete_point(deps.as_mut(), info.sender.as_ref()).unwrap();
    // This should error
    query_point(deps.as_ref()).unwrap_err();

    // Set Point as external user
    set_point(deps.as_mut(), &point, &external_user).unwrap();
    delete_point(deps.as_mut(), &external_user).unwrap();
    // This should error
    query_point(deps.as_ref()).unwrap_err();

    // Set Point as owner
    set_point(deps.as_mut(), &point, info.sender.as_ref()).unwrap();
    // Delete the point as external user
    delete_point(deps.as_mut(), &external_user).unwrap();
    // This should error
    query_point(deps.as_ref()).unwrap_err();
}

#[test]
fn test_restriction_restricted() {
    let (mut deps, info) = proper_initialization(PointRestriction::Restricted);

    let point = PointCoordinate::from_f64(10_f64, 10_f64, Some(10_f64));
    let point2 = PointCoordinate::from_f64(5_f64, 5_f64, Some(5_f64));
    let external_user = "external".to_string();
    let external_user2 = "external2".to_string();

    // Set point as owner
    set_point(deps.as_mut(), &point, info.sender.as_ref()).unwrap();
    delete_point(deps.as_mut(), info.sender.as_ref()).unwrap();
    // This should error
    query_point(deps.as_ref()).unwrap_err();

    // Set point as external user
    set_point(deps.as_mut(), &point, &external_user).unwrap();
    delete_point(deps.as_mut(), &external_user).unwrap();
    // This should error
    query_point(deps.as_ref()).unwrap_err();

    // Set point as owner and try to delete as external user
    set_point(deps.as_mut(), &point, info.sender.as_ref()).unwrap();
    // Try to modify it as external user
    set_point(deps.as_mut(), &point2, &external_user).unwrap_err();
    // Delete the point as external user, this should error
    delete_point(deps.as_mut(), &external_user).unwrap_err();

    query_point(deps.as_ref()).unwrap();

    // Set point as external user and try to delete as owner
    set_point(deps.as_mut(), &point, info.sender.as_ref()).unwrap();
    // Delete the point as external user, this will success as owner has permission to do anything
    delete_point(deps.as_mut(), info.sender.as_ref()).unwrap();

    query_point(deps.as_ref()).unwrap_err();

    // Set point as external user 1 and try to delete as external user 2
    set_point(deps.as_mut(), &point, &external_user).unwrap();
    // Delete the point as external user, this will error
    delete_point(deps.as_mut(), &external_user2).unwrap_err();

    query_point(deps.as_ref()).unwrap();
}

#[test]
fn test_query_data_owner() {
    let (mut deps, _) = proper_initialization(PointRestriction::Restricted);
    let external_user = "external".to_string();
    let external_user2 = "external2".to_string();
    let point = PointCoordinate::from_f64(10_f64, 10_f64, Some(10_f64));
    set_point(deps.as_mut(), &point, &external_user.clone()).unwrap();

    let res: GetDataOwnerResponse =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::GetDataOwner {}).unwrap()).unwrap();

    assert_eq!(
        res,
        GetDataOwnerResponse {
            owner: AndrAddr::from_string(external_user.clone())
        }
    );

    let res = delete_point(deps.as_mut(), &external_user2).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    delete_point(deps.as_mut(), &external_user).unwrap();

    query(deps.as_ref(), mock_env(), QueryMsg::GetDataOwner {}).unwrap_err();
}
