use crate::contract::{execute, instantiate, query};

use crate::state::CODE_ID;
use ado_base::ADOContract;
use andromeda_os::adodb::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_testing::testing::mock_querier::mock_dependencies_custom;
use common::{ado_base::AndromedaQuery, error::ContractError};
use cosmwasm_std::{
    attr, from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Response,
};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        kernel_address: None,
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
            kernel_address: None,
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
            kernel_address: None,
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
            kernel_address: None,
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
fn test_andr_get_query() {
    let mut deps = mock_dependencies_custom(&[]);

    CODE_ID
        .save(deps.as_mut().storage, "code_id", &1u64)
        .unwrap();

    let msg = QueryMsg::AndrQuery(AndromedaQuery::Get(Some(to_binary(&"code_id").unwrap())));

    let res: u64 = from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();

    assert_eq!(1u64, res);
}
