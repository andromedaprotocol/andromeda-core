use crate::contract::query;
use andromeda_math::point::{GetDataOwnerResponse, PointCoordinate, PointRestriction, QueryMsg};
use cosmwasm_std::{from_json, testing::mock_env, SignedDecimal};

use andromeda_std::{amp::AndrAddr, error::ContractError};

use super::mock::{delete_point, proper_initialization, query_point, set_point};

#[test]
fn test_instantiation() {
    proper_initialization(PointRestriction::Private);
}

#[test]
fn test_set_and_update_point() {
    let (mut deps, info) = proper_initialization(PointRestriction::Private);
    let point = PointCoordinate {
        x_coordinate: SignedDecimal::from_ratio(10, 1),
        y_coordinate: SignedDecimal::from_ratio(10, 1),
        z_coordinate: Some(SignedDecimal::from_ratio(10, 1)),
    };

    set_point(deps.as_mut(), &point, info.sender.as_ref()).unwrap();

    let query_res: PointCoordinate = query_point(deps.as_ref()).unwrap();

    assert_eq!(point, query_res);

    let point = PointCoordinate {
        x_coordinate: SignedDecimal::from_ratio(5, 1),
        y_coordinate: SignedDecimal::from_ratio(5, 1),
        z_coordinate: Some(SignedDecimal::from_ratio(5, 1)),
    };

    set_point(deps.as_mut(), &point, info.sender.as_ref()).unwrap();

    let query_res: PointCoordinate = query_point(deps.as_ref()).unwrap();

    assert_eq!(point, query_res);
}

#[test]
fn test_delete_point() {
    let (mut deps, info) = proper_initialization(PointRestriction::Private);
    let point = PointCoordinate {
        x_coordinate: SignedDecimal::from_ratio(10, 1),
        y_coordinate: SignedDecimal::from_ratio(10, 1),
        z_coordinate: Some(SignedDecimal::from_ratio(10, 1)),
    };

    set_point(deps.as_mut(), &point, info.sender.as_ref()).unwrap();
    delete_point(deps.as_mut(), info.sender.as_ref()).unwrap();
    query_point(deps.as_ref()).unwrap_err();
}

#[test]
fn test_restriction_private() {
    let (mut deps, info) = proper_initialization(PointRestriction::Private);

    let point = PointCoordinate {
        x_coordinate: SignedDecimal::from_ratio(10, 1),
        y_coordinate: SignedDecimal::from_ratio(10, 1),
        z_coordinate: Some(SignedDecimal::from_ratio(10, 1)),
    };
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

    let point = PointCoordinate {
        x_coordinate: SignedDecimal::from_ratio(10, 1),
        y_coordinate: SignedDecimal::from_ratio(10, 1),
        z_coordinate: Some(SignedDecimal::from_ratio(10, 1)),
    };
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

    let point = PointCoordinate {
        x_coordinate: SignedDecimal::from_ratio(10, 1),
        y_coordinate: SignedDecimal::from_ratio(10, 1),
        z_coordinate: Some(SignedDecimal::from_ratio(10, 1)),
    };
    let point2 = PointCoordinate {
        x_coordinate: SignedDecimal::from_ratio(5, 1),
        y_coordinate: SignedDecimal::from_ratio(5, 1),
        z_coordinate: Some(SignedDecimal::from_ratio(5, 1)),
    };
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
    let point = PointCoordinate {
        x_coordinate: SignedDecimal::from_ratio(10, 1),
        y_coordinate: SignedDecimal::from_ratio(10, 1),
        z_coordinate: Some(SignedDecimal::from_ratio(10, 1)),
    };
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
