#[cfg(test)]
use andromeda_std::testing::mock_querier::{mock_dependencies_custom, MOCK_KERNEL_CONTRACT};
use cosmwasm_std::{from_json, Uint128};

use crate::contract::{execute, instantiate, query};
use crate::state::{ACTION_FEES, CODE_ID, LATEST_VERSION, PUBLISHER, UNPUBLISHED_CODE_IDS};

use andromeda_std::error::ContractError;
use andromeda_std::os::adodb::{ADOVersion, ActionFee, ExecuteMsg, InstantiateMsg, QueryMsg};

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

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
            asset: "cw20:somecw20token".to_string(),
            receiver: None,
        },
        ActionFee {
            action: "action2".to_string(),
            amount: Uint128::from(2u128),
            asset: "native:uusd".to_string(),
            receiver: None,
        },
    ];

    // Empty ado_type
    let ado_version = ADOVersion::from_type("").with_version("0.1.0");
    let code_id = 1;
    let msg = ExecuteMsg::Publish {
        ado_type: ado_version.get_type(),
        version: ado_version.get_version(),
        code_id,
        action_fees: Some(action_fees.clone()),
        publisher: Some(owner.clone()),
    };

    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();

    assert_eq!(
        err,
        ContractError::InvalidADOType {
            msg: Some("ado_type can't be an empty string".to_string())
        }
    );

    // Invalid version
    let ado_version = ADOVersion::from_type("ado_type");
    let code_id = 1;
    let msg = ExecuteMsg::Publish {
        ado_type: ado_version.get_type(),
        version: ado_version.get_version(),
        code_id,
        action_fees: Some(action_fees.clone()),
        publisher: Some(owner.clone()),
    };

    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert!(err.is_err());

    // ado_type made only of spaces
    let ado_version = ADOVersion::from_type("       ").with_version("0.1.0");
    let code_id = 1;
    let msg = ExecuteMsg::Publish {
        ado_type: ado_version.get_type(),
        version: ado_version.get_version(),
        code_id,
        action_fees: Some(action_fees.clone()),
        publisher: Some(owner.clone()),
    };

    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();

    assert_eq!(
        err,
        ContractError::InvalidADOType {
            msg: Some("ado_type can't be an empty string".to_string())
        }
    );

    let ado_version = ADOVersion::from_type("ado_type").with_version("0.1.0");
    let code_id = 1;
    let msg = ExecuteMsg::Publish {
        ado_type: ado_version.get_type(),
        version: ado_version.get_version(),
        code_id,
        action_fees: Some(action_fees.clone()),
        publisher: Some(owner.clone()),
    };

    let resp = execute(deps.as_mut(), env.clone(), info, msg.clone());

    assert!(resp.is_ok());
    let publisher = PUBLISHER
        .load(deps.as_ref().storage, ado_version.as_str())
        .unwrap();
    assert_eq!(publisher, owner);

    let code_id = CODE_ID
        .load(deps.as_ref().storage, ado_version.as_str())
        .unwrap();
    assert_eq!(code_id, 1u64);

    let vers_code_id = LATEST_VERSION
        .load(deps.as_ref().storage, &ado_version.get_type())
        .unwrap();
    assert_eq!(vers_code_id.0, ado_version.get_version());
    assert_eq!(vers_code_id.1, code_id);

    // TEST ACTION FEE
    for action_fee in action_fees {
        let fee = ACTION_FEES
            .load(
                deps.as_ref().storage,
                &(ado_version.get_type(), action_fee.clone().action),
            )
            .unwrap();
        assert_eq!(fee, action_fee);
    }

    // Test unauthorised
    let unauth_info = mock_info("not_owner", &[]);
    let resp = execute(deps.as_mut(), env, unauth_info, msg);
    assert!(resp.is_err());
}

#[test]
fn test_unpublish() {
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
            asset: "cw20:somecw20token".to_string(),
            receiver: None,
        },
        ActionFee {
            action: "action2".to_string(),
            amount: Uint128::from(2u128),
            asset: "native:uusd".to_string(),
            receiver: None,
        },
    ];
    let ado_version = ADOVersion::from_type("ado_type").with_version("0.1.0");
    let code_id = 1;
    let msg = ExecuteMsg::Publish {
        ado_type: ado_version.get_type(),
        version: ado_version.get_version(),
        code_id,
        action_fees: Some(action_fees.clone()),
        publisher: Some(owner.clone()),
    };

    let resp = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());

    assert!(resp.is_ok());
    let publisher = PUBLISHER
        .load(deps.as_ref().storage, ado_version.as_str())
        .unwrap();
    assert_eq!(publisher, owner);

    let code_id = CODE_ID
        .load(deps.as_ref().storage, ado_version.as_str())
        .unwrap();
    assert_eq!(code_id, 1u64);

    let vers_code_id = LATEST_VERSION
        .load(deps.as_ref().storage, &ado_version.get_type())
        .unwrap();
    assert_eq!(vers_code_id.0, ado_version.get_version());
    assert_eq!(vers_code_id.1, code_id);

    // TEST ACTION FEE
    for action_fee in action_fees.clone() {
        let fee = ACTION_FEES
            .load(
                deps.as_ref().storage,
                &(ado_version.get_type(), action_fee.clone().action),
            )
            .unwrap();
        assert_eq!(fee, action_fee);
    }

    // Test unauthorised
    let unauth_info = mock_info("not_owner", &[]);
    let resp = execute(deps.as_mut(), env.clone(), unauth_info.clone(), msg);
    assert!(resp.is_err());

    // Test unpublish

    // Unauthorized

    let msg = ExecuteMsg::Unpublish {
        ado_type: ado_version.get_type(),
        version: ado_version.get_version(),
    };

    let err = execute(deps.as_mut(), env.clone(), unauth_info, msg).unwrap_err();

    assert_eq!(err, ContractError::Unauthorized {});

    // Invalid Version
    let invalid_ado_version = ADOVersion::from_type("ado_type").with_version("0.2.0");
    let msg = ExecuteMsg::Unpublish {
        ado_type: ado_version.get_type(),
        version: invalid_ado_version.get_version(),
    };

    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();

    assert_eq!(
        err,
        ContractError::InvalidADOVersion {
            msg: Some(String::from("Version not already published"))
        }
    );

    // Works
    let msg = ExecuteMsg::Unpublish {
        ado_type: ado_version.get_type(),
        version: ado_version.get_version(),
    };

    let resp = execute(deps.as_mut(), env.clone(), info.clone(), msg);

    assert!(resp.is_ok());

    let publisher = PUBLISHER.load(deps.as_ref().storage, ado_version.as_str());
    assert!(publisher.is_err());

    let code_id = CODE_ID.load(deps.as_ref().storage, ado_version.as_str());
    assert!(code_id.is_err());

    let vers_code_id = LATEST_VERSION.load(deps.as_ref().storage, &ado_version.get_type());
    assert!(vers_code_id.is_err());

    // Check on unpublished code ids
    let unpublished_code_ids = UNPUBLISHED_CODE_IDS.load(deps.as_ref().storage, 1).unwrap();
    // The code id that was originally published and then unpublished is 1
    // True means that it's unpublished
    assert!(unpublished_code_ids);

    // Make sure we can't republish an unpublished code id
    let ado_version = ADOVersion::from_type("ado_type").with_version("0.1.0");
    let msg = ExecuteMsg::Publish {
        ado_type: ado_version.get_type(),
        version: ado_version.get_version(),
        code_id: 1,
        action_fees: Some(action_fees.clone()),
        publisher: Some(owner.clone()),
    };

    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(err, ContractError::UnpublishedCodeID {});

    // Make sure we can't republish unpublished versions of corresponding ADO types
    // same type same version
    let ado_version = ADOVersion::from_type("ado_type").with_version("0.1.0");
    let code_id = 2;
    let msg = ExecuteMsg::Publish {
        ado_type: ado_version.get_type(),
        version: ado_version.get_version(),
        code_id,
        action_fees: Some(action_fees.clone()),
        publisher: Some(owner.clone()),
    };

    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(err, ContractError::UnpublishedVersion {});

    // Different type same version should work
    let ado_version = ADOVersion::from_type("ado_different_type").with_version("0.1.0");
    let code_id = 3;
    let msg = ExecuteMsg::Publish {
        ado_type: ado_version.get_type(),
        version: ado_version.get_version(),
        code_id,
        action_fees: Some(action_fees.clone()),
        publisher: Some(owner.clone()),
    };

    let resp = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert!(resp.is_ok());

    // Works with new code id and version of the same ado type
    let ado_version = ADOVersion::from_type("ado_type").with_version("0.2.0");
    let code_id = 2;
    let msg = ExecuteMsg::Publish {
        ado_type: ado_version.get_type(),
        version: ado_version.get_version(),
        code_id,
        action_fees: Some(action_fees.clone()),
        publisher: Some(owner.clone()),
    };

    let resp = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert!(resp.is_ok());

    // Publish 2 versions of the same ADO type
    let ado_version = ADOVersion::from_type("splitter").with_version("0.1.0");
    let code_id = 10;
    let msg = ExecuteMsg::Publish {
        ado_type: ado_version.get_type(),
        version: ado_version.get_version(),
        code_id,
        action_fees: Some(action_fees.clone()),
        publisher: Some(owner.clone()),
    };

    let resp = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert!(resp.is_ok());

    let ado_version = ADOVersion::from_type("splitter").with_version("0.2.0");
    let code_id = 11;
    let msg = ExecuteMsg::Publish {
        ado_type: ado_version.get_type(),
        version: ado_version.get_version(),
        code_id,
        action_fees: Some(action_fees.clone()),
        publisher: Some(owner.clone()),
    };

    let resp = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert!(resp.is_ok());

    let msg = ExecuteMsg::Unpublish {
        ado_type: "splitter".to_string(),
        version: "0.2.0".to_string(),
    };

    let resp = execute(deps.as_mut(), env.clone(), info.clone(), msg);

    assert!(resp.is_ok());

    let vers_code_id = LATEST_VERSION
        .load(deps.as_ref().storage, "splitter")
        .unwrap();
    // We published 2 versions of the splitter. The first being version 0.1.0, and the other 0.2.0.
    // While unpublishing the latest version, the penultimate version was set instead as the latest version
    assert_eq!(vers_code_id, ("0.1.0".to_string(), 10));

    // Let's try removing the penultimate version, the latest version should remain unchanged
    // First we'll publish a newer version so that we'll have two.
    let ado_version = ADOVersion::from_type("splitter").with_version("0.3.0");
    let code_id = 12;
    let msg = ExecuteMsg::Publish {
        ado_type: ado_version.get_type(),
        version: ado_version.get_version(),
        code_id,
        action_fees: Some(action_fees),
        publisher: Some(owner),
    };

    let resp = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    // Version 0.3.0 should now be the latest version, and removing 0.1.0 won't affect that.
    assert!(resp.is_ok());

    // Remove version 0.1.0
    let msg = ExecuteMsg::Unpublish {
        ado_type: "splitter".to_string(),
        version: "0.1.0".to_string(),
    };

    let resp = execute(deps.as_mut(), env.clone(), info.clone(), msg);

    assert!(resp.is_ok());

    let vers_code_id = LATEST_VERSION
        .load(deps.as_ref().storage, "splitter")
        .unwrap();
    assert_eq!(vers_code_id, ("0.3.0".to_string(), 12));

    // 0.3.0 is now the only version remaining for the splitter type, removing it should not result in errors
    let msg = ExecuteMsg::Unpublish {
        ado_type: "splitter".to_string(),
        version: "0.3.0".to_string(),
    };

    let resp = execute(deps.as_mut(), env, info, msg);
    assert!(resp.is_ok());

    // There shouldn't be any versions remaining since 0.3.0 was the last one
    let vers_code_id = LATEST_VERSION
        .may_load(deps.as_ref().storage, &ado_version.get_type())
        .unwrap();
    assert!(vers_code_id.is_none());
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
            asset: "cw20:somecw20token".to_string(),
            receiver: None,
        },
        ActionFee {
            action: "action2".to_string(),
            amount: Uint128::from(2u128),
            asset: "native:uusd".to_string(),
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
        .save(deps.as_mut().storage, ado_version.as_str(), &code_id)
        .unwrap();

    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert!(res.is_ok());

    // TEST ACTION FEE
    for action_fee in action_fees {
        let fee = ACTION_FEES
            .load(
                deps.as_ref().storage,
                &(ado_version.get_type(), action_fee.clone().action),
            )
            .unwrap();
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
        .save(deps.as_mut().storage, ado_version.as_str(), &code_id)
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
            &(ado_version.into_string(), action.to_string()),
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
        .save(deps.as_mut().storage, ado_version.as_str(), &code_id)
        .unwrap();

    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert!(res.is_ok());

    let publisher = PUBLISHER
        .load(deps.as_ref().storage, ado_version.as_str())
        .unwrap();
    assert_eq!(publisher, test_publisher);

    // Test unauthorised
    let unauth_info = mock_info("not_owner", &[]);
    let resp = execute(deps.as_mut(), env, unauth_info, msg).unwrap_err();
    assert_eq!(resp, ContractError::Unauthorized {});
}

#[test]
fn test_get_code_id() {
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

    let msg = ExecuteMsg::Publish {
        ado_type: ado_version.get_type(),
        version: ado_version.get_version(),
        code_id,
        action_fees: None,
        publisher: Some(owner),
    };
    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let query_msg = QueryMsg::CodeId {
        key: ado_version.clone().into_string(),
    };
    let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let value: u64 = from_json(res).unwrap();
    assert_eq!(value, code_id);

    let query_msg = QueryMsg::CodeId {
        key: ado_version.get_type(),
    };
    let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let value: u64 = from_json(res).unwrap();
    assert_eq!(value, code_id);

    let query_msg = QueryMsg::CodeId {
        key: format!("{}@latest", ado_version.get_type()),
    };
    let res = query(deps.as_ref(), env, query_msg).unwrap();
    let value: u64 = from_json(res).unwrap();
    assert_eq!(value, code_id);
}

#[test]
fn test_all_ado_types() {
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

    let mut code_id = 1;

    let ados = vec![
        ADOVersion::from_string("ado_type_1@0.1.0".to_string()),
        ADOVersion::from_string("ado_type_1@0.1.1".to_string()),
        ADOVersion::from_string("ado_type_2@0.1.0".to_string()),
    ];

    ados.iter().for_each(|ado_version| {
        let msg = ExecuteMsg::Publish {
            ado_type: ado_version.get_type(),
            version: ado_version.get_version(),
            code_id,
            action_fees: None,
            publisher: Some(owner.clone()),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        code_id += 1;
    });

    let query_msg = QueryMsg::AllADOTypes {
        start_after: None,
        limit: None,
    };
    let res = query(deps.as_ref(), env, query_msg).unwrap();
    let value: Vec<String> = from_json(res).unwrap();
    let expected = vec![
        "ado_type_1@0.1.0".to_string(),
        "ado_type_1@0.1.1".to_string(),
        "ado_type_2@0.1.0".to_string(),
    ];
    assert_eq!(value, expected);
}
