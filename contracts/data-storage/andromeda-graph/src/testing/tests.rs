use crate::testing::mock_querier::MOCK_POINT_CONTRACT;
use andromeda_data_storage::graph::{Coordinate, MapInfo, MapSize, StoredDate};
use andromeda_data_storage::graph::{CoordinateInfo, GetMapInfoResponse};
use andromeda_std::amp::AndrAddr;
use andromeda_std::error::ContractError;

use super::mock::{
    delete_user_coordinate, proper_initialization, query_all_points, query_map_info,
    query_max_point_number, query_user_coordinate, store_coordinate, store_user_coordinate,
    update_map,
};

#[test]
fn test_instantiation_z_allowed() {
    let (deps, _) = proper_initialization(MapInfo {
        map_size: MapSize {
            x_width: 10,
            y_width: 10,
            z_width: Some(10),
        },
        allow_negative: false,
        map_decimal: 5,
    });

    let res = query_map_info(deps.as_ref()).unwrap();
    assert_eq!(
        res,
        GetMapInfoResponse {
            map_info: MapInfo {
                map_size: MapSize {
                    x_width: 10,
                    y_width: 10,
                    z_width: Some(10),
                },
                allow_negative: false,
                map_decimal: 5,
            }
        }
    );
}

#[test]
fn test_instantiation_z_not_allowed() {
    let (deps, _) = proper_initialization(MapInfo {
        map_size: MapSize {
            x_width: 10,
            y_width: 10,
            z_width: None,
        },
        allow_negative: false,
        map_decimal: 5,
    });

    let res = query_map_info(deps.as_ref()).unwrap();
    assert_eq!(
        res,
        GetMapInfoResponse {
            map_info: MapInfo {
                map_size: MapSize {
                    x_width: 10,
                    y_width: 10,
                    z_width: None,
                },
                allow_negative: false,
                map_decimal: 5,
            }
        }
    );
}

#[test]
fn test_update_map_with_same_info() {
    let (mut deps, info) = proper_initialization(MapInfo {
        map_size: MapSize {
            x_width: 10,
            y_width: 10,
            z_width: None,
        },
        allow_negative: false,
        map_decimal: 5,
    });
    let err_res = update_map(
        deps.as_mut(),
        MapInfo {
            map_size: MapSize {
                x_width: 10,
                y_width: 10,
                z_width: None,
            },
            allow_negative: false,
            map_decimal: 5,
        },
        info.sender.as_ref(),
    )
    .unwrap_err();
    assert_eq!(
        err_res,
        ContractError::InvalidParameter {
            error: Some("Map already exists".to_string())
        }
    );
}

#[test]
fn test_store_coordinate_with_z_not_allowed() {
    let (mut deps, info) = proper_initialization(MapInfo {
        map_size: MapSize {
            x_width: 10,
            y_width: 10,
            z_width: None,
        },
        allow_negative: false,
        map_decimal: 5,
    });

    let err_res = store_coordinate(
        deps.as_mut(),
        Coordinate {
            x_coordinate: 9.12345_f64,
            y_coordinate: 2.12345_f64,
            z_coordinate: Some(4.12345_f64),
        },
        false,
        info.sender.as_ref(),
    )
    .unwrap_err();
    assert_eq!(
        err_res,
        ContractError::InvalidParameter {
            error: Some("Z-axis is not allowed".to_string())
        }
    );
}

#[test]
fn test_store_coordinate_with_z_allowed() {
    let (mut deps, info) = proper_initialization(MapInfo {
        map_size: MapSize {
            x_width: 10,
            y_width: 10,
            z_width: Some(10),
        },
        allow_negative: false,
        map_decimal: 5,
    });

    let err_res = store_coordinate(
        deps.as_mut(),
        Coordinate {
            x_coordinate: 9.12345_f64,
            y_coordinate: 2.12345_f64,
            z_coordinate: None,
        },
        false,
        info.sender.as_ref(),
    )
    .unwrap_err();
    assert_eq!(
        err_res,
        ContractError::InvalidParameter {
            error: Some("Z-axis is allowed".to_string())
        }
    );
}

#[test]
fn test_store_coordinate_with_wrong_range_disallow_negative_z_not_allowed() {
    let (mut deps, info) = proper_initialization(MapInfo {
        map_size: MapSize {
            x_width: 10,
            y_width: 10,
            z_width: None,
        },
        allow_negative: false,
        map_decimal: 5,
    });

    let err_res = store_coordinate(
        deps.as_mut(),
        Coordinate {
            x_coordinate: 9.12345_f64,
            y_coordinate: 12.12345_f64,
            z_coordinate: None,
        },
        false,
        info.sender.as_ref(),
    )
    .unwrap_err();
    assert_eq!(
        err_res,
        ContractError::InvalidParameter {
            error: Some("Wrong Y Coordinate Range".to_string())
        }
    );
}

#[test]
fn test_store_coordinate_with_wrong_range_disallow_negative_z_allowed() {
    let (mut deps, info) = proper_initialization(MapInfo {
        map_size: MapSize {
            x_width: 10,
            y_width: 10,
            z_width: Some(10),
        },
        allow_negative: false,
        map_decimal: 5,
    });

    let err_res = store_coordinate(
        deps.as_mut(),
        Coordinate {
            x_coordinate: 9.12345_f64,
            y_coordinate: 9.12345_f64,
            z_coordinate: Some(12.12345_f64),
        },
        false,
        info.sender.as_ref(),
    )
    .unwrap_err();
    assert_eq!(
        err_res,
        ContractError::InvalidParameter {
            error: Some("Wrong Z Coordinate Range".to_string())
        }
    );
}

#[test]
fn test_store_coordinate_with_wrong_range_allow_negative_z_not_allowed() {
    let (mut deps, info) = proper_initialization(MapInfo {
        map_size: MapSize {
            x_width: 10,
            y_width: 10,
            z_width: None,
        },
        allow_negative: true,
        map_decimal: 5,
    });

    let err_res = store_coordinate(
        deps.as_mut(),
        Coordinate {
            x_coordinate: -4.12345_f64,
            y_coordinate: 5.12345_f64,
            z_coordinate: None,
        },
        false,
        info.sender.as_ref(),
    )
    .unwrap_err();
    assert_eq!(
        err_res,
        ContractError::InvalidParameter {
            error: Some("Wrong Y Coordinate Range".to_string())
        }
    );
}

#[test]
fn test_store_coordinate_with_wrong_range_allow_negative_z_allowed() {
    let (mut deps, info) = proper_initialization(MapInfo {
        map_size: MapSize {
            x_width: 10,
            y_width: 10,
            z_width: Some(10),
        },
        allow_negative: true,
        map_decimal: 5,
    });

    let err_res = store_coordinate(
        deps.as_mut(),
        Coordinate {
            x_coordinate: -4.12345_f64,
            y_coordinate: 4.12345_f64,
            z_coordinate: Some(-12.12345_f64),
        },
        false,
        info.sender.as_ref(),
    )
    .unwrap_err();
    assert_eq!(
        err_res,
        ContractError::InvalidParameter {
            error: Some("Wrong Z Coordinate Range".to_string())
        }
    );
}

#[test]
fn test_store_coordinate_disallow_negative_and_update_map_timestamp_not_allowed() {
    let (mut deps, info) = proper_initialization(MapInfo {
        map_size: MapSize {
            x_width: 10,
            y_width: 10,
            z_width: None,
        },
        allow_negative: false,
        map_decimal: 5,
    });

    store_coordinate(
        deps.as_mut(),
        Coordinate {
            x_coordinate: 9.123456_f64,
            y_coordinate: 8.12345_f64,
            z_coordinate: None,
        },
        false,
        info.sender.as_ref(),
    )
    .unwrap();

    store_coordinate(
        deps.as_mut(),
        Coordinate {
            x_coordinate: 8.12345_f64,
            y_coordinate: 8.123458_f64,
            z_coordinate: None,
        },
        false,
        info.sender.as_ref(),
    )
    .unwrap();

    let max_point = query_max_point_number(deps.as_ref())
        .unwrap()
        .max_point_number;
    assert_eq!(max_point, 2);

    let all_points = query_all_points(deps.as_ref()).unwrap().points;
    assert_eq!(
        all_points,
        vec![
            (
                CoordinateInfo {
                    x: "9.12345".to_string(),
                    y: "8.12345".to_string(),
                    z: None,
                },
                StoredDate { timestamp: None }
            ),
            (
                CoordinateInfo {
                    x: "8.12345".to_string(),
                    y: "8.12345".to_string(),
                    z: None,
                },
                StoredDate { timestamp: None }
            ),
        ]
    );

    update_map(
        deps.as_mut(),
        MapInfo {
            map_size: MapSize {
                x_width: 100,
                y_width: 100,
                z_width: Some(100),
            },
            allow_negative: false,
            map_decimal: 5,
        },
        info.sender.as_ref(),
    )
    .unwrap();

    let all_points = query_all_points(deps.as_ref()).unwrap().points;
    assert_eq!(all_points, vec![]);

    let max_point = query_max_point_number(deps.as_ref())
        .unwrap()
        .max_point_number;
    assert_eq!(max_point, 0);
}

#[test]
fn test_store_coordinate_disallow_negative_timestamp_allowed() {
    let (mut deps, info) = proper_initialization(MapInfo {
        map_size: MapSize {
            x_width: 10,
            y_width: 10,
            z_width: None,
        },
        allow_negative: false,
        map_decimal: 5,
    });

    store_coordinate(
        deps.as_mut(),
        Coordinate {
            x_coordinate: 9.123456_f64,
            y_coordinate: 8.12345_f64,
            z_coordinate: None,
        },
        true,
        info.sender.as_ref(),
    )
    .unwrap();

    store_coordinate(
        deps.as_mut(),
        Coordinate {
            x_coordinate: 8.12345_f64,
            y_coordinate: 8.123458_f64,
            z_coordinate: None,
        },
        true,
        info.sender.as_ref(),
    )
    .unwrap();

    store_coordinate(
        deps.as_mut(),
        Coordinate {
            x_coordinate: 5_f64,
            y_coordinate: 8_f64,
            z_coordinate: None,
        },
        true,
        info.sender.as_ref(),
    )
    .unwrap();

    let max_point = query_max_point_number(deps.as_ref())
        .unwrap()
        .max_point_number;
    assert_eq!(max_point, 3);

    let all_points = query_all_points(deps.as_ref()).unwrap().points;
    assert_eq!(
        all_points,
        vec![
            (
                CoordinateInfo {
                    x: "9.12345".to_string(),
                    y: "8.12345".to_string(),
                    z: None,
                },
                StoredDate {
                    timestamp: Some(1_571_797_419_879_305_533),
                }
            ),
            (
                CoordinateInfo {
                    x: "8.12345".to_string(),
                    y: "8.12345".to_string(),
                    z: None,
                },
                StoredDate {
                    timestamp: Some(1_571_797_419_879_305_533),
                }
            ),
            (
                CoordinateInfo {
                    x: "5".to_string(),
                    y: "8".to_string(),
                    z: None,
                },
                StoredDate {
                    timestamp: Some(1_571_797_419_879_305_533),
                }
            ),
        ]
    );
}

#[test]
fn test_store_user_coordinate() {
    let (mut deps, info) = proper_initialization(MapInfo {
        map_size: MapSize {
            x_width: 100,
            y_width: 100,
            z_width: Some(100),
        },
        allow_negative: false,
        map_decimal: 5,
    });

    store_user_coordinate(
        deps.as_mut(),
        vec![AndrAddr::from_string(MOCK_POINT_CONTRACT.to_string())],
        info.sender.as_ref(),
    )
    .unwrap();

    let query_res: CoordinateInfo =
        query_user_coordinate(deps.as_ref(), AndrAddr::from_string("sender".to_string())).unwrap();
    assert_eq!(
        query_res,
        CoordinateInfo {
            x: "10".to_string(),
            y: "10".to_string(),
            z: Some("10".to_string()),
        },
    );

    delete_user_coordinate(
        deps.as_mut(),
        AndrAddr::from_string("sender".to_string()),
        "sender",
    )
    .unwrap();

    query_user_coordinate(deps.as_ref(), AndrAddr::from_string("sender".to_string())).unwrap_err();
}
