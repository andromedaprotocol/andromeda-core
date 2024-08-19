use crate::contract::{execute, query};
use andromeda_data_storage::matrix::{
    ExecuteMsg, GetMatrixResponse, Matrix, MatrixRestriction, QueryMsg,
};
use cosmwasm_std::{
    coin, from_json, testing::mock_env, BankMsg, CosmosMsg, Decimal, Response, SubMsg,
};

use andromeda_std::{
    ado_base::rates::{LocalRate, LocalRateType, LocalRateValue, PercentRate, Rate, RatesMessage},
    ado_contract::ADOContract,
    amp::{AndrAddr, Recipient},
    error::ContractError,
};

use super::mock::{
    delete_matrix, proper_initialization, query_matrix, store_matrix, store_matrix_with_funds,
};

#[test]
fn test_instantiation() {
    proper_initialization(MatrixRestriction::Private);
}

#[test]
fn test_store_and_update_matrix_with_key() {
    let (mut deps, info) = proper_initialization(MatrixRestriction::Private);
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
fn test_store_matrix_with_tax() {
    let (mut deps, info) = proper_initialization(MatrixRestriction::Private);
    let key = String::from("matrix1");
    let data = Matrix(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);
    let tax_recipient = "tax_recipient";

    // Set percent rates
    let set_percent_rate_msg = ExecuteMsg::Rates(RatesMessage::SetRate {
        action: "MatrixStoreMatrix".to_string(),
        rate: Rate::Local(LocalRate {
            rate_type: LocalRateType::Additive,
            recipients: vec![],
            value: LocalRateValue::Percent(PercentRate {
                percent: Decimal::one(),
            }),
            description: None,
        }),
    });

    let err = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        set_percent_rate_msg,
    )
    .unwrap_err();

    assert_eq!(err, ContractError::InvalidRate {});

    // Make sure sender is set as recipient when the recipients vector is empty
    let rate: Rate = Rate::Local(LocalRate {
        rate_type: LocalRateType::Additive,
        recipients: vec![],
        value: LocalRateValue::Flat(coin(20_u128, "uandr")),
        description: None,
    });

    let msg = ExecuteMsg::Rates(RatesMessage::SetRate {
        action: "StoreMatrix".to_string(),
        rate,
    });
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let queried_rates = ADOContract::default()
        .get_rates(deps.as_ref(), "StoreMatrix".to_string())
        .unwrap();
    assert_eq!(
        queried_rates.unwrap(),
        Rate::Local(LocalRate {
            rate_type: LocalRateType::Additive,
            recipients: vec![Recipient::new(AndrAddr::from_string("creator"), None)],
            value: LocalRateValue::Flat(coin(20_u128, "uandr")),
            description: None,
        })
    );

    let rate: Rate = Rate::Local(LocalRate {
        rate_type: LocalRateType::Additive,
        recipients: vec![Recipient {
            address: AndrAddr::from_string(tax_recipient.to_string()),
            msg: None,
            ibc_recovery_address: None,
        }],
        value: LocalRateValue::Flat(coin(20_u128, "uandr")),
        description: None,
    });

    // Set rates
    ADOContract::default()
        .set_rates(deps.as_mut().storage, "StoreMatrix", rate)
        .unwrap();

    // Sent the exact amount required for tax
    let res = store_matrix_with_funds(
        deps.as_mut(),
        &Some(key.clone()),
        &data,
        info.sender.as_ref(),
        coin(20_u128, "uandr".to_string()),
    )
    .unwrap();
    let expected_response: Response = Response::new()
        .add_submessage(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: tax_recipient.to_string(),
            amount: vec![coin(20, "uandr")],
        })))
        .add_attributes(vec![
            ("method", "store_matrix"),
            ("sender", "creator"),
            ("key", "matrix1"),
        ])
        .add_attribute("data", format!("{data:?}"));
    assert_eq!(expected_response, res);

    // Sent less than amount required for tax
    let err = store_matrix_with_funds(
        deps.as_mut(),
        &Some(key.clone()),
        &data,
        info.sender.as_ref(),
        coin(19_u128, "uandr".to_string()),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::InsufficientFunds {});

    // Sent more than required amount for tax
    let res = store_matrix_with_funds(
        deps.as_mut(),
        &Some(key.clone()),
        &data,
        info.sender.as_ref(),
        coin(200_u128, "uandr".to_string()),
    )
    .unwrap();
    let expected_response: Response = Response::new()
        .add_submessage(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: tax_recipient.to_string(),
            amount: vec![coin(20, "uandr")],
        })))
        // 200 was sent, but the tax is only 20, so we send back the difference
        .add_submessage(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "creator".to_string(),
            amount: vec![coin(180, "uandr")],
        })))
        .add_attributes(vec![
            ("method", "store_matrix"),
            ("sender", "creator"),
            ("key", "matrix1"),
        ])
        .add_attribute("data", format!("{data:?}"));
    assert_eq!(expected_response, res);
}

#[test]
fn test_store_and_update_matrix_without_key() {
    let (mut deps, info) = proper_initialization(MatrixRestriction::Private);
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
    let (mut deps, info) = proper_initialization(MatrixRestriction::Private);
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
    let (mut deps, info) = proper_initialization(MatrixRestriction::Private);
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
fn test_restriction_private() {
    let (mut deps, info) = proper_initialization(MatrixRestriction::Private);

    let key = Some("matrix1".to_string());
    let data = Matrix(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);
    let external_user = "external".to_string();

    // Store as owner
    store_matrix(deps.as_mut(), &key, &data, info.sender.as_ref()).unwrap();
    delete_matrix(deps.as_mut(), &key, info.sender.as_ref()).unwrap();
    // This should error, key is deleted
    query_matrix(deps.as_ref(), &key).unwrap_err();

    // Store as external user
    // This should error
    store_matrix(deps.as_mut(), &key, &data, &external_user).unwrap_err();
    // Store by owner so we can test delete for it
    store_matrix(deps.as_mut(), &key, &data, info.sender.as_ref()).unwrap();
    // Delete matrix set by owner by external user
    // This will error
    delete_matrix(deps.as_mut(), &key, &external_user).unwrap_err();

    // Key is still present
    query_matrix(deps.as_ref(), &key).unwrap();
}

#[test]
fn test_restriction_public() {
    let (mut deps, info) = proper_initialization(MatrixRestriction::Public);

    let key = Some("key".to_string());
    let data = Matrix(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);
    let external_user = "external".to_string();

    // Store as owner
    store_matrix(deps.as_mut(), &key, &data, info.sender.as_ref()).unwrap();
    delete_matrix(deps.as_mut(), &key, info.sender.as_ref()).unwrap();
    // This should error, key is deleted
    query_matrix(deps.as_ref(), &key).unwrap_err();

    // Store as external user
    store_matrix(deps.as_mut(), &key, &data, &external_user).unwrap();
    delete_matrix(deps.as_mut(), &key, &external_user).unwrap();
    // This should error, key is deleted
    query_matrix(deps.as_ref(), &key).unwrap_err();

    // Store as owner
    store_matrix(deps.as_mut(), &key, &data, info.sender.as_ref()).unwrap();
    // Delete the matrix as external user
    delete_matrix(deps.as_mut(), &key, &external_user).unwrap();
    // This should error, key is deleted
    query_matrix(deps.as_ref(), &key).unwrap_err();
}

#[test]
fn test_restriction_restricted() {
    let (mut deps, info) = proper_initialization(MatrixRestriction::Restricted);

    let key = Some("key".to_string());
    let data = Matrix(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);
    let data2 = Matrix(vec![
        vec![1, 2, 3, 1],
        vec![4, 5, 6, 1],
        vec![7, 8, 9, 1],
        vec![10, 11, 12, 1],
    ]);
    let external_user = "external".to_string();
    let external_user2 = "external2".to_string();

    // Store as owner
    store_matrix(deps.as_mut(), &key, &data, info.sender.as_ref()).unwrap();
    delete_matrix(deps.as_mut(), &key, info.sender.as_ref()).unwrap();
    // This should error, key is deleted
    query_matrix(deps.as_ref(), &key).unwrap_err();

    // Store as external user
    store_matrix(deps.as_mut(), &key, &data, &external_user).unwrap();
    delete_matrix(deps.as_mut(), &key, &external_user).unwrap();
    // This should error, key is deleted
    query_matrix(deps.as_ref(), &key).unwrap_err();

    // Store as owner and try to delete as external user
    store_matrix(deps.as_mut(), &key, &data, info.sender.as_ref()).unwrap();
    // Try to modify it as external user
    store_matrix(deps.as_mut(), &key, &data2, &external_user).unwrap_err();
    // Delete as external user, this should error
    delete_matrix(deps.as_mut(), &key, &external_user).unwrap_err();
    // Key is still present
    query_matrix(deps.as_ref(), &key).unwrap();

    let key = Some("key2".to_string());
    // Store as external user and try to delete as owner
    store_matrix(deps.as_mut(), &key, &data, &external_user).unwrap();
    // Delete as external user, this will success as owner has permission to do anything
    delete_matrix(deps.as_mut(), &key, info.sender.as_ref()).unwrap();
    // Key is not present, this will error
    query_matrix(deps.as_ref(), &key).unwrap_err();

    let key = Some("key3".to_string());
    // Store as external user 1 and try to delete as external user 2
    store_matrix(deps.as_mut(), &key, &data, &external_user).unwrap();
    // Delete as external user, this will error
    delete_matrix(deps.as_mut(), &key, &external_user2).unwrap_err();
    // Key is present
    query_matrix(deps.as_ref(), &key).unwrap();
}

#[test]
fn test_query_all_key() {
    let (mut deps, info) = proper_initialization(MatrixRestriction::Private);

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
    let (mut deps, _) = proper_initialization(MatrixRestriction::Restricted);

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
