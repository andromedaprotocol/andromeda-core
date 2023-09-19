#[cfg(test)]
use andromeda_std::testing::mock_querier::MOCK_KERNEL_CONTRACT;
use cosmwasm_schema::schemars::Map;
use cosmwasm_std::{
    from_binary,
    testing::{mock_env, mock_info, MockApi, MockStorage},
    Deps, DepsMut, MessageInfo, OwnedDeps,
};

use crate::{
    contract::{execute, instantiate, query},
    state::DEFAULT_KEY,
};
use andromeda_data_storage::primitive::{
    ExecuteMsg, GetValueResponse, InstantiateMsg, Primitive, PrimitiveRestriction, QueryMsg,
};

use andromeda_std::{
    error::ContractError,
    testing::mock_querier::{mock_dependencies_custom, WasmMockQuerier},
};

use cosmwasm_std::Response;

fn proper_initialization(
    restriction: PrimitiveRestriction,
) -> (
    OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
    MessageInfo,
) {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        restriction,
    };
    let env = mock_env();
    instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
    (deps, info)
}

fn query_value_helper(
    deps: Deps,
    name: &Option<String>,
) -> Result<GetValueResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetValue { key: name.clone() });
    match res {
        Ok(res) => Ok(from_binary(&res).unwrap()),
        Err(err) => Err(err),
    }
}

fn set_value_helper(
    deps: DepsMut<'_>,
    key: &Option<String>,
    value: &Primitive,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::SetValue {
        key: key.clone(),
        value: value.clone(),
    };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

fn delete_value_helper(
    deps: DepsMut<'_>,
    key: &Option<String>,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::DeleteValue { key: key.clone() };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

#[test]
fn test_instantiation() {
    proper_initialization(PrimitiveRestriction::Private);
}

#[test]
fn set_and_update_value_with_key() {
    let (mut deps, info) = proper_initialization(PrimitiveRestriction::Private);
    let key = Some(String::from("key"));
    let value = Primitive::String("value".to_string());
    set_value_helper(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();

    let query_res: GetValueResponse = query_value_helper(deps.as_ref(), &key).unwrap();

    assert_eq!(
        GetValueResponse {
            key: key.clone().unwrap_or("default".into()),
            value
        },
        query_res
    );

    let value = Primitive::String("value2".to_string());
    set_value_helper(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();

    let query_res: GetValueResponse = query_value_helper(deps.as_ref(), &key).unwrap();

    assert_eq!(
        GetValueResponse {
            key: key.unwrap_or("default".into()),
            value
        },
        query_res
    );
}

#[test]
fn set_and_update_value_without_key() {
    let (mut deps, info) = proper_initialization(PrimitiveRestriction::Private);
    let key = None;
    let value = Primitive::String("value".to_string());
    set_value_helper(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();

    let query_res: GetValueResponse = query_value_helper(deps.as_ref(), &key).unwrap();

    assert_eq!(
        GetValueResponse {
            key: key.clone().unwrap_or("default".into()),
            value
        },
        query_res
    );

    let value = Primitive::String("value2".to_string());
    set_value_helper(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();

    let query_res: GetValueResponse = query_value_helper(deps.as_ref(), &key).unwrap();

    assert_eq!(
        GetValueResponse {
            key: key.unwrap_or("default".into()),
            value
        },
        query_res
    );
}

// #[test]
// fn cannot_set_nested_vector_primitive() {
//     let mut deps = mock_dependencies_custom(&[]);

//     let msg = InstantiateMsg {
//         kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
//         owner: None,
//         vfs_name: None,
//     };
//     let info = mock_info("creator", &[]);

//     // we can just call .unwrap() to assert this was a success
//     let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//     let msg = ExecuteMsg::SetValue {
//         key: None,
//         value: Primitive::Vec(vec![Primitive::Vec(vec![])]),
//     };
//     let res: Result<Response, ContractError> = execute(deps.as_mut(), mock_env(), info, msg);
//     assert_eq!(ContractError::InvalidPrimitive {}, res.unwrap_err());
// }

#[test]
fn delete_value() {
    let (mut deps, info) = proper_initialization(PrimitiveRestriction::Private);
    // Without Key
    let value = Primitive::String("value".to_string());
    set_value_helper(deps.as_mut(), &None, &value, info.sender.as_ref()).unwrap();
    delete_value_helper(deps.as_mut(), &None, info.sender.as_ref()).unwrap();
    query_value_helper(deps.as_ref(), &None).unwrap_err();

    // With key
    let key = Some("key".to_string());
    set_value_helper(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();
    delete_value_helper(deps.as_mut(), &key, info.sender.as_ref()).unwrap();
    query_value_helper(deps.as_ref(), &key).unwrap_err();
}

#[test]
fn test_restriction_private() {
    let (mut deps, info) = proper_initialization(PrimitiveRestriction::Private);

    let key = Some("key".to_string());
    let value = Primitive::String("value".to_string());
    let external_user = "external".to_string();

    // Set Value as owner
    set_value_helper(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();
    delete_value_helper(deps.as_mut(), &key, info.sender.as_ref()).unwrap();
    // This should error, key is deleted
    query_value_helper(deps.as_ref(), &key).unwrap_err();

    // Set Value as external user
    // This should error
    set_value_helper(deps.as_mut(), &key, &value, &external_user).unwrap_err();
    // Set a value by owner so we can test delete for it
    set_value_helper(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();
    // Delete value set by owner by external user
    // This will error
    delete_value_helper(deps.as_mut(), &key, &external_user).unwrap_err();

    // Key is still present
    query_value_helper(deps.as_ref(), &key).unwrap();
}

#[test]
fn test_restriction_public() {
    let (mut deps, info) = proper_initialization(PrimitiveRestriction::Public);

    let key = Some("key".to_string());
    let value = Primitive::String("value".to_string());
    let external_user = "external".to_string();

    // Set Value as owner
    set_value_helper(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();
    delete_value_helper(deps.as_mut(), &key, info.sender.as_ref()).unwrap();
    // This should error, key is deleted
    query_value_helper(deps.as_ref(), &key).unwrap_err();

    // Set Value as external user
    set_value_helper(deps.as_mut(), &key, &value, &external_user).unwrap();
    delete_value_helper(deps.as_mut(), &key, &external_user).unwrap();
    // This should error, key is deleted
    query_value_helper(deps.as_ref(), &key).unwrap_err();

    // Set Value as owner
    set_value_helper(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();
    // Delete the value as external user
    delete_value_helper(deps.as_mut(), &key, &external_user).unwrap();
    // This should error, key is deleted
    query_value_helper(deps.as_ref(), &key).unwrap_err();
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
    set_value_helper(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();
    delete_value_helper(deps.as_mut(), &key, info.sender.as_ref()).unwrap();
    // This should error, key is deleted
    query_value_helper(deps.as_ref(), &key).unwrap_err();

    // Set Value as external user
    set_value_helper(deps.as_mut(), &key, &value, &external_user).unwrap();
    delete_value_helper(deps.as_mut(), &key, &external_user).unwrap();
    // This should error, key is deleted
    query_value_helper(deps.as_ref(), &key).unwrap_err();

    // Set Value as owner and try to delete as external user
    set_value_helper(deps.as_mut(), &key, &value, info.sender.as_ref()).unwrap();
    // Try to modify it as external user
    set_value_helper(deps.as_mut(), &key, &value2, &external_user).unwrap_err();
    // Delete the value as external user, this should error
    delete_value_helper(deps.as_mut(), &key, &external_user).unwrap_err();
    // Key is still present
    query_value_helper(deps.as_ref(), &key).unwrap();

    let key = Some("key2".to_string());
    // Set Value as external user and try to delete as owner
    set_value_helper(deps.as_mut(), &key, &value, &external_user).unwrap();
    // Delete the value as external user, this will success as owner has permission to do anything
    delete_value_helper(deps.as_mut(), &key, info.sender.as_ref()).unwrap();
    // Key is not present, this will error
    query_value_helper(deps.as_ref(), &key).unwrap_err();

    let key = Some("key3".to_string());
    // Set Value as external user 1 and try to delete as external user 2
    set_value_helper(deps.as_mut(), &key, &value, &external_user).unwrap();
    // Delete the value as external user, this will error
    delete_value_helper(deps.as_mut(), &key, &external_user2).unwrap_err();
    // Key is present
    query_value_helper(deps.as_ref(), &key).unwrap();
}

#[test]
fn query_all_key() {
    let (mut deps, info) = proper_initialization(PrimitiveRestriction::Private);

    let keys: Vec<String> = vec!["key1".into(), "key2".into()];
    let value = Primitive::String("value".to_string());
    for key in keys.clone() {
        set_value_helper(deps.as_mut(), &Some(key), &value, info.sender.as_ref()).unwrap();
    }

    let res: Vec<String> =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::AllKeys {}).unwrap()).unwrap();

    assert_eq!(res, keys)
}

#[test]
fn set_object() {
    let (mut deps, info) = proper_initialization(PrimitiveRestriction::Private);

    let mut map = Map::new();
    map.insert("key".to_string(), Primitive::Bool(true));

    set_value_helper(
        deps.as_mut(),
        &None,
        &Primitive::Object(map.clone()),
        info.sender.as_ref(),
    )
    .unwrap();

    let query_res: GetValueResponse = query_value_helper(deps.as_ref(), &None).unwrap();

    assert_eq!(
        GetValueResponse {
            key: DEFAULT_KEY.to_string(),
            value: Primitive::Object(map.clone())
        },
        query_res
    );
}
