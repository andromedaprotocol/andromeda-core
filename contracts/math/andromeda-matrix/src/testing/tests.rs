use crate::contract::query;
use andromeda_math::matrix::{GetMatrixResponse, Matrix, QueryMsg};
use cosmwasm_std::{from_json, testing::mock_env};

use andromeda_std::{amp::AndrAddr, error::ContractError};

use super::mock::{delete_matrix, proper_initialization, query_matrix, store_matrix};

pub const AUTHORIZED_OPERATOR: &str = "authorized_operator";
pub const UNAUTHORIZED_OPERATOR: &str = "unauthorized_operator";

#[test]
fn test_instantiation() {
    proper_initialization(None);
}

#[test]
fn test_store_and_update_matrix_with_key() {
    let (mut deps, info) = proper_initialization(None);
    let key = String::from("matrix1");
    let data = Matrix(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);
    store_matrix(
        deps.as_mut(),
        &Some(key.clone()),
        &data,
        info.sender.as_ref(),
    )
    .unwrap();

    let query_res: GetMatrixResponse = query_matrix(deps.as_ref(), &Some(key.clone())).unwrap();

    assert_eq!(
        GetMatrixResponse {
            key: key.clone(),
            data
        },
        query_res
    );

    let data = Matrix(vec![vec![1, 2, 5], vec![4, 5, 6], vec![7, 8, 9]]);
    store_matrix(
        deps.as_mut(),
        &Some(key.clone()),
        &data,
        info.sender.as_ref(),
    )
    .unwrap();

    let query_res: GetMatrixResponse = query_matrix(deps.as_ref(), &Some(key.clone())).unwrap();

    assert_eq!(GetMatrixResponse { key, data }, query_res);
}

#[test]
fn test_store_and_update_matrix_without_key() {
    let (mut deps, info) = proper_initialization(None);
    let key = None;
    let data = Matrix(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);
    store_matrix(deps.as_mut(), &key, &data, info.sender.as_ref()).unwrap();

    let query_res: GetMatrixResponse = query_matrix(deps.as_ref(), &key).unwrap();

    assert_eq!(
        GetMatrixResponse {
            key: key.clone().unwrap_or("default".into()),
            data,
        },
        query_res
    );

    let data = Matrix(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 19]]);
    store_matrix(deps.as_mut(), &key, &data, info.sender.as_ref()).unwrap();

    let query_res: GetMatrixResponse = query_matrix(deps.as_ref(), &key).unwrap();

    assert_eq!(
        GetMatrixResponse {
            key: "default".to_string(),
            data,
        },
        query_res
    );
}

#[test]
fn test_store_matrix_invalid() {
    let (mut deps, info) = proper_initialization(None);
    let key = None;
    let data = Matrix(vec![vec![1, 2, 3, 4], vec![4, 5, 6], vec![7, 8, 9]]);
    let err_res = store_matrix(deps.as_mut(), &key, &data, info.sender.as_ref()).unwrap_err();

    assert_eq!(
        ContractError::CustomError {
            msg: "All rows in the matrix must have the same number of columns".to_string()
        },
        err_res,
    );
}

#[test]
fn test_delete_matrix() {
    let (mut deps, info) = proper_initialization(None);
    // Without Key
    let data = Matrix(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);
    store_matrix(deps.as_mut(), &None, &data, info.sender.as_ref()).unwrap();
    delete_matrix(deps.as_mut(), &None, info.sender.as_ref()).unwrap();
    query_matrix(deps.as_ref(), &None).unwrap_err();

    // With key
    let key = Some("matrix1".to_string());
    store_matrix(deps.as_mut(), &key, &data, info.sender.as_ref()).unwrap();
    delete_matrix(deps.as_mut(), &key, info.sender.as_ref()).unwrap();
    query_matrix(deps.as_ref(), &key).unwrap_err();
}

#[test]
fn test_authorization() {
    let (mut deps, info) =
        proper_initialization(Some(vec![AndrAddr::from_string(AUTHORIZED_OPERATOR)]));

    let key = Some("matrix1".to_string());
    let data = Matrix(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);

    // Store as owner
    store_matrix(deps.as_mut(), &key, &data, info.sender.as_ref()).unwrap();
    let matrix = query_matrix(deps.as_ref(), &key).unwrap();
    assert_eq!(
        matrix,
        GetMatrixResponse {
            key: "matrix1".to_string(),
            data: Matrix(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]])
        }
    );

    delete_matrix(deps.as_mut(), &key, info.sender.as_ref()).unwrap();
    // This should error, key is deleted
    query_matrix(deps.as_ref(), &key).unwrap_err();

    // Store as external user
    // This should error
    let err = store_matrix(deps.as_mut(), &key, &data, UNAUTHORIZED_OPERATOR).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});
}

#[test]
fn test_query_all_key() {
    let (mut deps, info) = proper_initialization(None);

    let keys: Vec<String> = vec!["key1".into(), "key2".into()];
    let data = Matrix(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);
    for key in keys.clone() {
        store_matrix(deps.as_mut(), &Some(key), &data, info.sender.as_ref()).unwrap();
    }

    let res: Vec<String> =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::AllKeys {}).unwrap()).unwrap();

    assert_eq!(res, keys)
}

#[test]
fn test_query_owner_keys() {
    let (mut deps, _) = proper_initialization(None);

    let keys: Vec<String> = vec!["1".into(), "2".into()];
    let data = Matrix(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);
    let sender = "sender1".to_string();
    for key in keys.clone() {
        store_matrix(
            deps.as_mut(),
            &Some(format!("{sender}-{key}")),
            &data,
            &sender,
        )
        .unwrap();
    }

    let sender = "sender2".to_string();
    for key in keys {
        store_matrix(
            deps.as_mut(),
            &Some(format!("{sender}-{key}")),
            &data,
            &sender,
        )
        .unwrap();
    }

    let res: Vec<String> =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::AllKeys {}).unwrap()).unwrap();
    assert!(res.len() == 4, "Not all keys added");

    let res: Vec<String> = from_json(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::OwnerKeys {
                owner: AndrAddr::from_string("sender1"),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert!(res.len() == 2, "assertion failed {res:?}", res = res);

    let res: Vec<String> = from_json(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::OwnerKeys {
                owner: AndrAddr::from_string("sender2"),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert!(res.len() == 2, "assertion failed {res:?}", res = res);
}
