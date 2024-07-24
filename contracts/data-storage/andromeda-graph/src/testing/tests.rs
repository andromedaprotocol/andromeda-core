use andromeda_data_storage::graph::{CoordinateResponse, GetMapInfoResponse};
use andromeda_data_storage::graph::{
    MapInfo, MapSize, Coordinate,
};
use andromeda_std::error::ContractError;

use super::mock::{
    proper_initialization, update_map, query_map_info, store_coordinate, query_all_points, query_max_point,
};

#[test]
fn test_instantiation() {
    let (deps, _) = proper_initialization(
        MapInfo { 
            map_size: MapSize { x_length: 10, y_length: 10 }, 
            allow_negative: false, 
            map_decimal: 5, 
        },
    );

    let res = query_map_info(deps.as_ref()).unwrap();
    assert_eq!(
        res,
        GetMapInfoResponse {
            map_info: MapInfo { 
                map_size: MapSize { x_length: 10, y_length: 10 }, 
                allow_negative: false, 
                map_decimal: 5, 
            }
        }
    );
}

#[test]
fn test_update_map_with_same_info() {
    let (mut deps, info) = proper_initialization(
        MapInfo { 
            map_size: MapSize { x_length: 10, y_length: 10 }, 
            allow_negative: false, 
            map_decimal: 5, 
        },
    );
    let err_res = update_map(
        deps.as_mut(), 
        MapInfo { 
            map_size: MapSize { x_length: 10, y_length: 10 }, 
            allow_negative: false, 
            map_decimal: 5, 
        },
        info.sender.as_ref(),
    ).unwrap_err();
    assert_eq!(err_res, ContractError::InvalidParameter { error: Some("Map Info is same as existed one".to_string()) });
}


#[test]
fn test_store_coordinate_with_wrong_range_disallow_negative() {
    let (mut deps, info) = proper_initialization(
        MapInfo { 
            map_size: MapSize { x_length: 10, y_length: 10 }, 
            allow_negative: false, 
            map_decimal: 5, 
        },
    );
    
    let err_res = store_coordinate(
        deps.as_mut(), 
        Coordinate { 
            x_coordinate: 9.12345_f64, 
            y_coordinate: 12.12345_f64,
        }, 
        info.sender.as_ref()
    ).unwrap_err();
    assert_eq!(err_res, ContractError::InvalidParameter { error: Some("Wrong Y Coordinate Range".to_string())});
}

#[test]
fn test_store_coordinate_with_wrong_range_allow_negative() {
    let (mut deps, info) = proper_initialization(
        MapInfo { 
            map_size: MapSize { x_length: 10, y_length: 10 }, 
            allow_negative: true, 
            map_decimal: 5, 
        },
    );
    
    let err_res = store_coordinate(
        deps.as_mut(), 
        Coordinate { 
            x_coordinate: -4.12345_f64, 
            y_coordinate: 5.12345_f64,
        }, 
        info.sender.as_ref()
    ).unwrap_err();
    assert_eq!(err_res, ContractError::InvalidParameter { error: Some("Wrong Y Coordinate Range".to_string())});
}

#[test]
fn test_store_coordinate_disallow_negative_and_update_map() {
    let (mut deps, info) = proper_initialization(
        MapInfo { 
            map_size: MapSize { x_length: 10, y_length: 10 }, 
            allow_negative: false, 
            map_decimal: 5, 
        },
    );
    
    store_coordinate(
        deps.as_mut(), 
        Coordinate { 
            x_coordinate: 9.123456_f64, 
            y_coordinate: 8.12345_f64,
        }, 
        info.sender.as_ref()
    ).unwrap();

    store_coordinate(
        deps.as_mut(), 
        Coordinate { 
            x_coordinate: 8.12345_f64, 
            y_coordinate: 8.123458_f64,
        }, 
        info.sender.as_ref()
    ).unwrap();

    let max_point = query_max_point(deps.as_ref()).unwrap().max_point;
    assert_eq!(max_point, 2);

    let all_points = query_all_points(deps.as_ref()).unwrap().points;
    assert_eq!(all_points, vec![
        CoordinateResponse { x: "9.12345".to_string(), y: "8.12345".to_string() },
        CoordinateResponse { x: "8.12345".to_string(), y: "8.12345".to_string() },
    ]);

    update_map(
        deps.as_mut(), 
        MapInfo { 
            map_size: MapSize { x_length: 100, y_length: 100 }, 
            allow_negative: false, 
            map_decimal: 5, 
        },
        info.sender.as_ref(),
    ).unwrap();

    let all_points = query_all_points(deps.as_ref()).unwrap().points;
    assert_eq!(all_points, vec![]);

    let max_point = query_max_point(deps.as_ref()).unwrap().max_point;
    assert_eq!(max_point, 0);
}
