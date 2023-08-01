#[cfg(test)]
use andromeda_std::testing::mock_querier::{mock_dependencies_custom, MOCK_KERNEL_CONTRACT};
use cosmwasm_std::Uint128;

use crate::contract::{execute, instantiate};
use crate::state::{ACTION_FEES, CODE_ID, PUBLISHER, LATEST_VERSION};

use andromeda_std::ado_contract::ADOContract;
use andromeda_std::error::ContractError;
use andromeda_std::os::adodb::{ActionFee, ExecuteMsg, InstantiateMsg, ADOVersion};

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

    let action_fees = vec![
        ActionFee {
            action: "action".to_string(),
            amount: Uint128::from(1u128),
            asset: "somecw20token".to_string(),
            receiver: None,
        },
        ActionFee {
            action: "action2".to_string(),
            amount: Uint128::from(2u128),
            asset: "uusd".to_string(),
            receiver: None,
        },
    ];

    let ado_version = ADOVersion::from_type("ado_type").with_version("0.1.0");
    let code_id = 1;
    let msg = ExecuteMsg::Publish {
        ado_type: ado_version.get_type(),
        version: ado_version.get_version(),
        code_id: code_id,
        action_fees: Some(action_fees.clone()),
        publisher: Some(owner.clone()),
    };

    let resp = execute(deps.as_mut(), env.clone(), info, msg.clone());

    assert!(resp.is_ok());
    let publisher = PUBLISHER
        .load(deps.as_ref().storage, &ado_version.as_str())
        .unwrap();
    assert_eq!(publisher, owner);

    let code_id = CODE_ID.load(deps.as_ref().storage, &ado_version.as_str()).unwrap();
    assert_eq!(code_id, 1u64);

    let vers_code_id = LATEST_VERSION
        .load(
            deps.as_ref().storage,
            &ado_version.get_type(),
        )
        .unwrap();
    assert_eq!(vers_code_id.0, ado_version.clone().into_string());
    assert_eq!(vers_code_id.1, code_id);


    // TEST ACTION FEE
    for action_fee in action_fees.clone() {
        let fee = ACTION_FEES.load(
            deps.as_ref().storage,
            &(ado_version.clone().into_string(), action_fee.clone().action)
        ).unwrap();
        assert_eq!(fee, action_fee);
    }

    // Test unauthorised
    let unauth_info = mock_info("not_owner", &[]);
    let resp = execute(deps.as_mut(), env, unauth_info, msg);
    assert!(resp.is_err());
}

#[test]
fn test_update_action_fees() {
    let owner = String::from("owner");
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info(owner.as_str(), &[]);
    let ado_version = ADOVersion::from_type("ado_type").with_version("0.1.0");
    let code_id = 1;

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

    let action_fees = vec![
        ActionFee {
            action: "action".to_string(),
            amount: Uint128::from(1u128),
            asset: "somecw20token".to_string(),
            receiver: None,
        },
        ActionFee {
            action: "action2".to_string(),
            amount: Uint128::from(2u128),
            asset: "uusd".to_string(),
            receiver: None,
        },
    ];

    let msg = ExecuteMsg::UpdateActionFees {
        action_fees: action_fees.clone(),
        ado_type: ado_version.clone().into_string(),
    };

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();
    assert_eq!(
        res,
        ContractError::InvalidADOVersion {
            msg: Some("ADO type does not exist".to_string())
        }
    );

    CODE_ID
        .save(deps.as_mut().storage, &ado_version.as_str(), &code_id)
        .unwrap();

    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert!(res.is_ok());

    // TEST ACTION FEE
    for action_fee in action_fees.clone() {
        let fee = ACTION_FEES.load(
            deps.as_ref().storage,
            &(ado_version.clone().into_string(), action_fee.clone().action)
        ).unwrap();
        assert_eq!(fee, action_fee);
    }

    // Test unauthorised
    let unauth_info = mock_info("not_owner", &[]);
    let resp = execute(deps.as_mut(), env, unauth_info, msg);
    assert!(resp.is_err());
}

#[test]
fn test_remove_action_fees() {
    let owner = String::from("owner");
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info(owner.as_str(), &[]);
    let ado_version = ADOVersion::from_type("ado_type").with_version("0.1.0");
    let code_id = 1;
    let action = "action";
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

    let msg = ExecuteMsg::RemoveActionFees {
        ado_type: ado_version.clone().into_string(),
        actions: vec![action.to_string(), "not_an_action".to_string()], // Add extra action to ensure no error when a false action is provided
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();
    assert_eq!(
        res,
        ContractError::InvalidADOVersion {
            msg: Some("ADO type does not exist".to_string())
        }
    );

    CODE_ID
        .save(deps.as_mut().storage, &ado_version.clone().as_str(), &code_id)
        .unwrap();

    ACTION_FEES
        .save(
            deps.as_mut().storage,
            &(ado_version.clone().into_string(), action.to_string()),
            &ActionFee::new(action.to_string(), "uusd".to_string(), Uint128::from(1u128)),
        )
        .unwrap();

    let unauth_info = mock_info("not_owner", &[]);
    let res = execute(deps.as_mut(), env.clone(), unauth_info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    let res = execute(deps.as_mut(), env, info, msg);
    assert!(res.is_ok());

    let fee = ACTION_FEES
        .may_load(
            deps.as_ref().storage,
            &(ado_version.clone().into_string(), action.to_string()),
        )
        .unwrap();

    assert!(fee.is_none());
}

#[test]
fn test_update_publisher() {
    let owner = String::from("owner");
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info(owner.as_str(), &[]);
    let ado_version = ADOVersion::from_type("ado_type").with_version("0.1.0");
    let code_id = 1;
    let test_publisher = "new_publisher".to_string();

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

    let msg = ExecuteMsg::UpdatePublisher {
        ado_type: ado_version.clone().into_string(),
        publisher: test_publisher.clone(),
    };

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();
    assert_eq!(
        res,
        ContractError::InvalidADOVersion {
            msg: Some("ADO type does not exist".to_string())
        }
    );

    CODE_ID
        .save(deps.as_mut().storage, &ado_version.as_str(), &code_id)
        .unwrap();

    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert!(res.is_ok());

    let publisher = PUBLISHER
        .load(deps.as_ref().storage, &ado_version.as_str())
        .unwrap();
    assert_eq!(publisher, test_publisher);

    // Test unauthorised
    let unauth_info = mock_info("not_owner", &[]);
    let resp = execute(deps.as_mut(), env, unauth_info, msg).unwrap_err();
    assert_eq!(resp, ContractError::Unauthorized {});
}
