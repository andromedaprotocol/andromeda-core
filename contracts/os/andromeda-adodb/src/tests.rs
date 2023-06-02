#[cfg(test)]
use andromeda_std::testing::mock_querier::{mock_dependencies_custom, MOCK_KERNEL_CONTRACT};

use crate::contract::{execute, instantiate};

use andromeda_std::ado_contract::ADOContract;
use andromeda_std::error::ContractError;
use andromeda_std::os::adodb::{ExecuteMsg, InstantiateMsg};

use cosmwasm_std::{
    attr,
    testing::{mock_dependencies, mock_env, mock_info},
    Response,
};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
    };
    let env = mock_env();

    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_update_code_id() {
    let owner = String::from("owner");
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info(owner.as_str(), &[]);

    instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info(&owner, &[]),
        InstantiateMsg {
            kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
            owner: None,
        },
    )
    .unwrap();

    let msg = ExecuteMsg::UpdateCodeId {
        code_id_key: "address_list".to_string(),
        code_id: 1u64,
    };

    let resp = execute(deps.as_mut(), env, info, msg).unwrap();

    let expected = Response::new().add_attributes(vec![
        attr("action", "add_update_code_id"),
        attr("code_id_key", "address_list"),
        attr("code_id", "1"),
    ]);

    assert_eq!(resp, expected);
}

#[test]
fn test_update_code_id_operator() {
    let owner = String::from("owner");
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info(owner.as_str(), &[]);

    instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info(&owner, &[]),
        InstantiateMsg {
            kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
            owner: None,
        },
    )
    .unwrap();

    let operator = String::from("operator");
    ADOContract::default()
        .execute_update_operators(deps.as_mut(), info, vec![operator.clone()])
        .unwrap();

    let msg = ExecuteMsg::UpdateCodeId {
        code_id_key: "address_list".to_string(),
        code_id: 1u64,
    };

    let info = mock_info(&operator, &[]);
    let resp = execute(deps.as_mut(), env, info, msg).unwrap();

    let expected = Response::new().add_attributes(vec![
        attr("action", "add_update_code_id"),
        attr("code_id_key", "address_list"),
        attr("code_id", "1"),
    ]);

    assert_eq!(resp, expected);
}

#[test]
fn test_update_code_id_unauthorized() {
    let owner = String::from("owner");
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info(&owner, &[]),
        InstantiateMsg {
            kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
            owner: None,
        },
    )
    .unwrap();

    let msg = ExecuteMsg::UpdateCodeId {
        code_id_key: "address_list".to_string(),
        code_id: 1u64,
    };

    let info = mock_info("not_owner", &[]);
    let resp = execute(deps.as_mut(), env, info, msg);

    assert_eq!(ContractError::Unauthorized {}, resp.unwrap_err());
}

#[test]
fn test_publish() {
    let owner = String::from("owner");
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info(owner.as_str(), &[]);

    instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info(&owner, &[]),
        InstantiateMsg {
            kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
            owner: None,
        },
    )
    .unwrap();

    let msg = ExecuteMsg::Publish {
        ado_type: "ado_type".to_string(),
        version: "0.1.0".to_string(),
        code_id: 1,
        action_fees: None,
        publisher: Some(owner),
    };

    let resp = execute(deps.as_mut(), env, info, msg);

    assert!(resp.is_ok())
}
