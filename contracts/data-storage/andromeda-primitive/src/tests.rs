#[cfg(test)]
use andromeda_std::testing::mock_querier::MOCK_KERNEL_CONTRACT;
use cosmwasm_std::{
    from_binary,
    testing::{mock_env, mock_info},
    to_binary, CosmosMsg, Deps, Empty, ReplyOn, SubMsg, WasmMsg,
};

use crate::{
    contract::{execute, instantiate, query},
    state::DEFAULT_KEY,
};
use andromeda_data_storage::primitive::{
    ExecuteMsg, GetValueResponse, InstantiateMsg, Primitive, QueryMsg,
};
use andromeda_std::os::vfs::ExecuteMsg as VFSExecuteMsg;

use andromeda_std::{error::ContractError, testing::mock_querier::mock_dependencies_custom};

use cosmwasm_std::Response;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        vfs_name: None,
    };
    let env = mock_env();

    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn vfs_initialization() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        vfs_name: "primitive".to_string().into(),
    };
    let env = mock_env();

    let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
    let register_submsg: SubMsg<Empty> = SubMsg {
        id: 1001,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "vfs_contract".to_string(),
            msg: to_binary(&VFSExecuteMsg::AddParentPath {
                name: "primitive".into(),
                parent_address: info.sender,
            })
            .unwrap(),
            funds: vec![],
        }),
        reply_on: ReplyOn::Error,
        gas_limit: None,
    };

    let expected = Response::new().add_submessage(register_submsg);
    assert_eq!(expected.messages.to_vec(), res.messages.to_vec());
}

fn query_value_helper(deps: Deps, name: Option<String>) -> Result<GetValueResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetValue { key: name });
    match res {
        Ok(res) => Ok(from_binary(&res).unwrap()),
        Err(err) => Err(err),
    }
}

#[test]
fn set_and_update_value_with_key() {
    let mut deps = mock_dependencies_custom(&[]);

    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        vfs_name: None,
    };
    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::SetValue {
        key: Some("test1".to_string()),
        value: Primitive::String("value1".to_string()),
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        Response::new()
            .add_attribute("method", "set_value")
            .add_attribute("sender", "creator")
            .add_attribute("key", "test1")
            .add_attribute("value", "String(\"value1\")"),
        res
    );

    let query_res: GetValueResponse =
        query_value_helper(deps.as_ref(), Some("test1".to_string())).unwrap();

    assert_eq!(
        GetValueResponse {
            key: "test1".to_string(),
            value: Primitive::String("value1".to_string())
        },
        query_res
    );

    // Update the value to something else
    let msg = ExecuteMsg::SetValue {
        key: Some("test1".to_string()),
        value: Primitive::String("value2".to_string()),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res: GetValueResponse =
        query_value_helper(deps.as_ref(), Some("test1".to_string())).unwrap();

    assert_eq!(
        GetValueResponse {
            key: "test1".to_string(),
            value: Primitive::String("value2".to_string())
        },
        query_res
    );
}

#[test]
fn set_and_update_value_without_key() {
    let mut deps = mock_dependencies_custom(&[]);

    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        vfs_name: None,
    };
    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::SetValue {
        key: None,
        value: Primitive::String("value1".to_string()),
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        Response::new()
            .add_attribute("method", "set_value")
            .add_attribute("sender", "creator")
            .add_attribute("key", DEFAULT_KEY)
            .add_attribute("value", "String(\"value1\")"),
        res
    );

    let query_res: GetValueResponse = query_value_helper(deps.as_ref(), None).unwrap();

    assert_eq!(
        GetValueResponse {
            key: DEFAULT_KEY.to_string(),
            value: Primitive::String("value1".to_string())
        },
        query_res
    );

    // Update the value to something else
    let msg = ExecuteMsg::SetValue {
        key: None,
        value: Primitive::String("value2".to_string()),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res: GetValueResponse = query_value_helper(deps.as_ref(), None).unwrap();

    assert_eq!(
        GetValueResponse {
            key: DEFAULT_KEY.to_string(),
            value: Primitive::String("value2".to_string())
        },
        query_res
    );
}

#[test]
fn cannot_set_nested_vector_primitive() {
    let mut deps = mock_dependencies_custom(&[]);

    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        vfs_name: None,
    };
    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::SetValue {
        key: None,
        value: Primitive::Vec(vec![Primitive::Vec(vec![])]),
    };
    let res: Result<Response, ContractError> = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::InvalidPrimitive {}, res.unwrap_err());
}

#[test]
fn delete_value_with_key() {
    let mut deps = mock_dependencies_custom(&[]);

    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        vfs_name: None,
    };
    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::SetValue {
        key: Some("test1".to_string()),
        value: Primitive::String("value1".to_string()),
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let query_res: GetValueResponse =
        query_value_helper(deps.as_ref(), Some("test1".to_string())).unwrap();

    assert_eq!(
        GetValueResponse {
            key: "test1".to_string(),
            value: Primitive::String("value1".to_string())
        },
        query_res
    );

    let msg = ExecuteMsg::DeleteValue {
        key: Some("test1".to_string()),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res,
        Response::new()
            .add_attribute("method", "delete_value")
            .add_attribute("sender", "creator")
            .add_attribute("key", "test1")
    );
    let query_res = query_value_helper(deps.as_ref(), Some("test1".to_string()));
    assert!(query_res.is_err());
}

#[test]
fn delete_value_without_key() {
    let mut deps = mock_dependencies_custom(&[]);

    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        vfs_name: None,
    };
    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::SetValue {
        key: None,
        value: Primitive::String("value1".to_string()),
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let query_res: GetValueResponse = query_value_helper(deps.as_ref(), None).unwrap();

    assert_eq!(
        GetValueResponse {
            key: DEFAULT_KEY.to_string(),
            value: Primitive::String("value1".to_string())
        },
        query_res
    );

    let msg = ExecuteMsg::DeleteValue { key: None };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res,
        Response::new()
            .add_attribute("method", "delete_value")
            .add_attribute("sender", "creator")
            .add_attribute("key", DEFAULT_KEY)
    );
    let query_res = &query_value_helper(deps.as_ref(), None);
    assert!(query_res.is_err());
}

#[test]
fn non_creator_cannot_set_value() {
    let mut deps = mock_dependencies_custom(&[]);

    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        vfs_name: None,
    };
    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let user1 = mock_info("user1", &[]);
    let msg = ExecuteMsg::SetValue {
        key: Some("test1".to_string()),
        value: Primitive::String("value1".to_string()),
    };
    let res: Result<Response, ContractError> = execute(deps.as_mut(), mock_env(), user1, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn non_creator_cannot_delete_value() {
    let mut deps = mock_dependencies_custom(&[]);

    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        vfs_name: None,
    };
    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::SetValue {
        key: None,
        value: Primitive::String("value1".to_string()),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let user1 = mock_info("user1", &[]);
    let msg = ExecuteMsg::DeleteValue { key: None };
    let res: Result<Response, ContractError> = execute(deps.as_mut(), mock_env(), user1, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn query_all_key() {
    let mut deps = mock_dependencies_custom(&[]);

    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        vfs_name: None,
    };
    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let keys: Vec<String> = vec!["key1".into(), "key2".into()];
    for key in keys.clone() {
        let msg = ExecuteMsg::SetValue {
            key: Some(key),
            value: Primitive::String("value".to_string()),
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    let res: Vec<String> =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::AllKeys {}).unwrap()).unwrap();

    assert_eq!(res, keys)
}
