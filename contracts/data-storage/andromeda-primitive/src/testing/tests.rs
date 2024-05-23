use crate::contract::query;
use andromeda_data_storage::primitive::{
    GetValueResponse, Primitive, PrimitiveRestriction, QueryMsg,
};
use cosmwasm_std::{coin, from_json, testing::mock_env, Binary};

use andromeda_std::{
    ado_base::rates::{LocalRate, LocalRateType, LocalRateValue, Rate},
    ado_contract::ADOContract,
    amp::{AndrAddr, Recipient},
    error::ContractError,
};

use super::mock::{
    delete_value, proper_initialization, query_value, set_value, set_value_with_funds,
};

#[test]
fn test_instantiation() {
    proper_initialization(PrimitiveRestriction::Private);
}

#[test]
fn test_set_and_update_value_with_key() {
    let (mut deps, info) = proper_initialization(PrimitiveRestriction::Private);
    let key = String::from("key");
    let value = Primitive::String("value".to_string());
    set_value(
        deps.as_mut(),
        &Some(key.clone()),
        &value,
        info.sender.as_ref(),
    )
    .unwrap();

    let query_res: GetValueResponse = query_value(deps.as_ref(), &Some(key.clone())).unwrap();

    assert_eq!(
        GetValueResponse {
            key: key.clone(),
            value
        },
        query_res
    );

    let value = Primitive::String("value2".to_string());
    set_value(
        deps.as_mut(),
        &Some(key.clone()),
        &value,
        info.sender.as_ref(),
    )
    .unwrap();

    let query_res: GetValueResponse = query_value(deps.as_ref(), &Some(key.clone())).unwrap();

    assert_eq!(GetValueResponse { key, value }, query_res);
}

#[test]
fn test_set_value_with_tax() {
    let (mut deps, info) = proper_initialization(PrimitiveRestriction::Private);
    let key = String::from("key");
    let value = Primitive::String("value".to_string());
    let tax_recipient = "tax_recipient";

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
        .set_rates(deps.as_mut().storage, "PrimitiveSetValue", rate)
        .unwrap();

    // Sent the exact amount required for tax
    let _res = set_value_with_funds(
        deps.as_mut(),
        &Some(key.clone()),
        &value,
        info.sender.as_ref(),
        coin(20_u128, "uandr".to_string()),
    )
    .unwrap();

    // Sent less than amount required for tax
    let err = set_value_with_funds(
        deps.as_mut(),
        &Some(key.clone()),
        &value,
        info.sender.as_ref(),
        coin(19_u128, "uandr".to_string()),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::InsufficientFunds {});

    // Sent more than required amount for tax
    let _res = set_value_with_funds(
        deps.as_mut(),
        &Some(key.clone()),
        &value,
        info.sender.as_ref(),
        coin(200_u128, "uandr".to_string()),
    )
    .unwrap();
}

#[test]
fn test_set_and_update_value_without_key() {
    let (mut deps, info) = proper_initialization(PrimitiveRestriction::Private);
    let key = None;
    let value = Primitive::String("value".to_string());
    set_value(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();

    let query_res: GetValueResponse = query_value(deps.as_ref(), &key).unwrap();

    assert_eq!(
        GetValueResponse {
            key: key.clone().unwrap_or("default".into()),
            value
        },
        query_res
    );

    let value = Primitive::String("value2".to_string());
    set_value(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();

    let query_res: GetValueResponse = query_value(deps.as_ref(), &key).unwrap();

    assert_eq!(
        GetValueResponse {
            key: "default".to_string(),
            value
        },
        query_res
    );
}

#[test]
fn test_set_value_invalid() {
    let (mut deps, info) = proper_initialization(PrimitiveRestriction::Private);
    let key = String::from("key");
    // Empty String should error
    let value = Primitive::String("".to_string());
    let err = set_value(
        deps.as_mut(),
        &Some(key.clone()),
        &value,
        info.sender.as_ref(),
    )
    .unwrap_err();

    assert_eq!(err, ContractError::EmptyString {});

    // Empty denom should error
    let value = Primitive::Coin(coin(1_u128, "".to_string()));
    let err = set_value(
        deps.as_mut(),
        &Some(key.clone()),
        &value,
        info.sender.as_ref(),
    )
    .unwrap_err();

    assert_eq!(err, ContractError::InvalidDenom {});

    // Empty Binary should error
    let value = Primitive::Binary(Binary::default());
    let err = set_value(
        deps.as_mut(),
        &Some(key.clone()),
        &value,
        info.sender.as_ref(),
    )
    .unwrap_err();

    assert_eq!(err, ContractError::EmptyString {});
}

#[test]
fn test_delete_value() {
    let (mut deps, info) = proper_initialization(PrimitiveRestriction::Private);
    // Without Key
    let value = Primitive::String("value".to_string());
    set_value(deps.as_mut(), &None, &value, info.sender.as_ref()).unwrap();
    delete_value(deps.as_mut(), &None, info.sender.as_ref()).unwrap();
    query_value(deps.as_ref(), &None).unwrap_err();

    // With key
    let key = Some("key".to_string());
    set_value(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();
    delete_value(deps.as_mut(), &key, info.sender.as_ref()).unwrap();
    query_value(deps.as_ref(), &key).unwrap_err();
}

#[test]
fn test_restriction_private() {
    let (mut deps, info) = proper_initialization(PrimitiveRestriction::Private);

    let key = Some("key".to_string());
    let value = Primitive::String("value".to_string());
    let external_user = "external".to_string();

    // Set Value as owner
    set_value(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();
    delete_value(deps.as_mut(), &key, info.sender.as_ref()).unwrap();
    // This should error, key is deleted
    query_value(deps.as_ref(), &key).unwrap_err();

    // Set Value as external user
    // This should error
    set_value(deps.as_mut(), &key, &value, &external_user).unwrap_err();
    // Set a value by owner so we can test delete for it
    set_value(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();
    // Delete value set by owner by external user
    // This will error
    delete_value(deps.as_mut(), &key, &external_user).unwrap_err();

    // Key is still present
    query_value(deps.as_ref(), &key).unwrap();
}

#[test]
fn test_restriction_public() {
    let (mut deps, info) = proper_initialization(PrimitiveRestriction::Public);

    let key = Some("key".to_string());
    let value = Primitive::String("value".to_string());
    let external_user = "external".to_string();

    // Set Value as owner
    set_value(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();
    delete_value(deps.as_mut(), &key, info.sender.as_ref()).unwrap();
    // This should error, key is deleted
    query_value(deps.as_ref(), &key).unwrap_err();

    // Set Value as external user
    set_value(deps.as_mut(), &key, &value, &external_user).unwrap();
    delete_value(deps.as_mut(), &key, &external_user).unwrap();
    // This should error, key is deleted
    query_value(deps.as_ref(), &key).unwrap_err();

    // Set Value as owner
    set_value(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();
    // Delete the value as external user
    delete_value(deps.as_mut(), &key, &external_user).unwrap();
    // This should error, key is deleted
    query_value(deps.as_ref(), &key).unwrap_err();
}

#[test]
fn test_restriction_restricted() {
    let (mut deps, info) = proper_initialization(PrimitiveRestriction::Restricted);

    let key = Some("key".to_string());
    let value = Primitive::String("value".to_string());
    let value2 = Primitive::String("value2".to_string());
    let external_user = "external".to_string();
    let external_user2 = "external2".to_string();

    // Set Value as owner
    set_value(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();
    delete_value(deps.as_mut(), &key, info.sender.as_ref()).unwrap();
    // This should error, key is deleted
    query_value(deps.as_ref(), &key).unwrap_err();

    // Set Value as external user
    set_value(deps.as_mut(), &key, &value, &external_user).unwrap();
    delete_value(deps.as_mut(), &key, &external_user).unwrap();
    // This should error, key is deleted
    query_value(deps.as_ref(), &key).unwrap_err();

    // Set Value as owner and try to delete as external user
    set_value(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();
    // Try to modify it as external user
    set_value(deps.as_mut(), &key, &value2, &external_user).unwrap_err();
    // Delete the value as external user, this should error
    delete_value(deps.as_mut(), &key, &external_user).unwrap_err();
    // Key is still present
    query_value(deps.as_ref(), &key).unwrap();

    let key = Some("key2".to_string());
    // Set Value as external user and try to delete as owner
    set_value(deps.as_mut(), &key, &value, &external_user).unwrap();
    // Delete the value as external user, this will success as owner has permission to do anything
    delete_value(deps.as_mut(), &key, info.sender.as_ref()).unwrap();
    // Key is not present, this will error
    query_value(deps.as_ref(), &key).unwrap_err();

    let key = Some("key3".to_string());
    // Set Value as external user 1 and try to delete as external user 2
    set_value(deps.as_mut(), &key, &value, &external_user).unwrap();
    // Delete the value as external user, this will error
    delete_value(deps.as_mut(), &key, &external_user2).unwrap_err();
    // Key is present
    query_value(deps.as_ref(), &key).unwrap();
}

#[test]
fn test_query_all_key() {
    let (mut deps, info) = proper_initialization(PrimitiveRestriction::Private);

    let keys: Vec<String> = vec!["key1".into(), "key2".into()];
    let value = Primitive::String("value".to_string());
    for key in keys.clone() {
        set_value(deps.as_mut(), &Some(key), &value, info.sender.as_ref()).unwrap();
    }

    let res: Vec<String> =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::AllKeys {}).unwrap()).unwrap();

    assert_eq!(res, keys)
}

#[test]
fn test_query_owner_keys() {
    let (mut deps, _) = proper_initialization(PrimitiveRestriction::Restricted);

    let keys: Vec<String> = vec!["1".into(), "2".into()];
    let value = Primitive::String("value".to_string());
    let sender = "sender1".to_string();
    for key in keys.clone() {
        set_value(
            deps.as_mut(),
            &Some(format!("{sender}-{key}")),
            &value,
            &sender,
        )
        .unwrap();
    }

    let sender = "sender2".to_string();
    for key in keys {
        set_value(
            deps.as_mut(),
            &Some(format!("{sender}-{key}")),
            &value,
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
