use cosmwasm_std::{coins, BankMsg, Response, StdError, Uint128};

use crate::contract::{execute, instantiate};
use andromeda_finance::weighted_splitter::{AddressWeight, ExecuteMsg, InstantiateMsg};
use andromeda_testing::testing::mock_querier::{
    mock_dependencies_custom, MOCK_ADDRESSLIST_CONTRACT,
};
use common::{
    ado_base::{
        modules::Module, recipient::ADORecipient, AndromedaMsg,
        InstantiateMsg as BaseInstantiateMsg,
    },
    app::AndrAddress,
    error::ContractError,
};

use crate::contract::query;
use crate::state::SPLITTER;
use ado_base::ADOContract;
use andromeda_finance::weighted_splitter::{
    GetSplitterConfigResponse, GetUserWeightResponse, QueryMsg, Splitter,
};
use common::ado_base::recipient::Recipient;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{attr, from_binary, Coin, CosmosMsg, SubMsg};

#[test]
fn test_modules() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        modules: Some(vec![Module {
            module_type: "address_list".to_string(),
            is_mutable: false,
            address: AndrAddress {
                identifier: MOCK_ADDRESSLIST_CONTRACT.to_owned(),
            },
        }]),
        recipients: vec![AddressWeight {
            recipient: Recipient::from_string(String::from("Some Address")),
            weight: Uint128::new(100),
        }],
    };
    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    let expected_res = Response::new()
        .add_attribute("action", "register_module")
        .add_attribute("module_idx", "1")
        .add_attribute("method", "instantiate")
        .add_attribute("type", "weighted-splitter");
    assert_eq!(expected_res, res);

    let msg = ExecuteMsg::Send {};
    let info = mock_info("anyone", &coins(100, "uusd"));

    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());

    assert_eq!(
        ContractError::Std(StdError::generic_err(
            "Querier contract error: InvalidAddress"
        ),),
        res.unwrap_err()
    );

    let info = mock_info("sender", &coins(100, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "Some Address".to_string(),
                amount: coins(100, "uusd"),
            })
            .add_attribute("action", "send")
            .add_attribute("sender", "sender"),
        res
    );
}

#[test]
fn test_update_app_contract() {
    let mut deps = mock_dependencies_custom(&[]);

    let modules: Vec<Module> = vec![Module {
        module_type: "address_list".to_string(),
        address: AndrAddress {
            identifier: MOCK_ADDRESSLIST_CONTRACT.to_owned(),
        },
        is_mutable: false,
    }];

    let info = mock_info("app_contract", &[]);
    let msg = InstantiateMsg {
        modules: Some(modules),
        recipients: vec![
            AddressWeight {
                recipient: Recipient::from_string(String::from("Some Address")),
                weight: Uint128::new(50),
            },
            AddressWeight {
                recipient: Recipient::ADO(ADORecipient {
                    address: AndrAddress {
                        identifier: "e".to_string(),
                    },
                    msg: None,
                }),
                weight: Uint128::new(50),
            },
        ],
    };

    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::UpdateAppContract {
        address: "app_contract".to_string(),
    });

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "update_app_contract")
            .add_attribute("address", "app_contract"),
        res
    );
}

#[test]
fn test_update_app_contract_invalid_recipient() {
    let mut deps = mock_dependencies_custom(&[]);

    let modules: Vec<Module> = vec![Module {
        module_type: "address_list".to_string(),
        address: AndrAddress {
            identifier: MOCK_ADDRESSLIST_CONTRACT.to_owned(),
        },
        is_mutable: false,
    }];

    let info = mock_info("app_contract", &[]);
    let msg = InstantiateMsg {
        modules: Some(modules),
        recipients: vec![AddressWeight {
            recipient: Recipient::ADO(ADORecipient {
                address: AndrAddress {
                    identifier: "z".to_string(),
                },
                msg: None,
            }),
            weight: Uint128::new(100),
        }],
    };

    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::UpdateAppContract {
        address: "app_contract".to_string(),
    });

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidComponent {
            name: "z".to_string()
        },
        res.unwrap_err()
    );
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        recipients: vec![AddressWeight {
            recipient: Recipient::from_string(String::from("Some Address")),
            weight: Uint128::new(1),
        }],
        modules: None,
    };
    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_execute_update_lock() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let owner = "creator";

    let splitter = Splitter {
        recipients: vec![],
        locked: false,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let lock = true;
    let msg = ExecuteMsg::UpdateLock { lock };
    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            deps_mut.api,
            mock_info(owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                operators: None,
                modules: None,
                primitive_contract: None,
            },
        )
        .unwrap();

    let info = mock_info("incorrect_owner", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

    let info = mock_info(owner, &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        Response::default().add_attributes(vec![
            attr("action", "update_lock"),
            attr("locked", lock.to_string())
        ]),
        res
    );

    //check result
    let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
    assert_eq!(splitter.locked, lock);
}

#[test]
fn test_execute_remove_recipient() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let owner = "creator";

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr1")),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr2")),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr3")),
            weight: Uint128::new(50),
        },
    ];

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            deps_mut.api,
            mock_info(owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                operators: None,
                modules: None,
                primitive_contract: None,
            },
        )
        .unwrap();

    let info = mock_info(owner, &[]);

    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let splitter = Splitter {
        recipients: recipient,
        locked: false,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::RemoveRecipient {
        recipient: Recipient::from_string(String::from("addr1")),
    };
    // Try removing a user that isn't in the list
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::RemoveRecipient {
        recipient: Recipient::from_string(String::from("addr1")),
    };

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::UserNotFound {});

    let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
    let expected_splitter = Splitter {
        recipients: vec![
            AddressWeight {
                recipient: Recipient::from_string(String::from("addr3")),
                weight: Uint128::new(50),
            },
            AddressWeight {
                recipient: Recipient::from_string(String::from("addr2")),
                weight: Uint128::new(60),
            },
        ],
        locked: false,
    };
    assert_eq!(expected_splitter, splitter);
    assert_eq!(
        Response::default().add_attributes(vec![attr("action", "removed_recipient")]),
        res
    );

    // check result
    let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
    assert_eq!(
        splitter.recipients[0],
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr3")),
            weight: Uint128::new(50),
        }
    );
    assert_eq!(splitter.recipients.len(), 2);
}

#[test]
fn test_execute_remove_recipient_unauthorized() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let owner = "creator";

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr1")),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr2")),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr3")),
            weight: Uint128::new(50),
        },
    ];
    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient,
    };

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            deps_mut.api,
            mock_info(owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                operators: None,
                modules: None,
                primitive_contract: None,
            },
        )
        .unwrap();

    let info = mock_info("incorrect_owner", &[]);
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_update_recipient_weight() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner = "creator";

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr1")),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr2")),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr3")),
            weight: Uint128::new(50),
        },
    ];
    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            deps_mut.api,
            mock_info(owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                operators: None,
                modules: None,
                primitive_contract: None,
            },
        )
        .unwrap();

    let info = mock_info("incorrect_owner", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

    let info = mock_info(owner, &[]);

    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let splitter = Splitter {
        recipients: recipient.clone(),
        locked: false,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Locked contract
    let splitter = Splitter {
        recipients: recipient.clone(),
        locked: true,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();
    let msg = ExecuteMsg::UpdateRecipientWeight {
        recipient: AddressWeight {
            recipient: Recipient::from_string(String::from("addr1")),
            weight: Uint128::new(100),
        },
    };
    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(err, ContractError::ContractLocked {});
    // Works
    let splitter = Splitter {
        recipients: recipient,
        locked: false,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();
    let msg = ExecuteMsg::UpdateRecipientWeight {
        recipient: AddressWeight {
            recipient: Recipient::from_string(String::from("addr1")),
            weight: Uint128::new(100),
        },
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        Response::default().add_attributes(vec![attr("action", "updated_recipient_weight")]),
        res
    );
    let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
    let expected_splitter = Splitter {
        recipients: vec![
            AddressWeight {
                recipient: Recipient::from_string(String::from("addr1")),
                weight: Uint128::new(100),
            },
            AddressWeight {
                recipient: Recipient::from_string(String::from("addr2")),
                weight: Uint128::new(60),
            },
            AddressWeight {
                recipient: Recipient::from_string(String::from("addr3")),
                weight: Uint128::new(50),
            },
        ],
        locked: false,
    };
    assert_eq!(expected_splitter, splitter);
}

#[test]
fn test_update_recipient_weight_user_not_found() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner = "creator";

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr1")),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr2")),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr3")),
            weight: Uint128::new(50),
        },
    ];
    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            deps_mut.api,
            mock_info(owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                operators: None,
                modules: None,
                primitive_contract: None,
            },
        )
        .unwrap();

    let info = mock_info("incorrect_owner", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

    let info = mock_info(owner, &[]);

    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let splitter = Splitter {
        recipients: recipient,
        locked: false,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // User not found
    let msg = ExecuteMsg::UpdateRecipientWeight {
        recipient: AddressWeight {
            recipient: Recipient::from_string(String::from("addr4")),
            weight: Uint128::new(100),
        },
    };
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::UserNotFound {});
}

#[test]

fn test_update_recipient_weight_invalid_weight() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner = "creator";

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr1")),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr2")),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr3")),
            weight: Uint128::new(50),
        },
    ];
    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            deps_mut.api,
            mock_info(owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                operators: None,
                modules: None,
                primitive_contract: None,
            },
        )
        .unwrap();

    let info = mock_info("incorrect_owner", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

    let info = mock_info(owner, &[]);

    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let splitter = Splitter {
        recipients: recipient,
        locked: false,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::UpdateRecipientWeight {
        recipient: AddressWeight {
            recipient: Recipient::from_string(String::from("addr1")),
            weight: Uint128::zero(),
        },
    };
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::InvalidWeight {});
}

#[test]
fn test_execute_add_recipient() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let owner = "creator";

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr1")),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr2")),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr3")),
            weight: Uint128::new(50),
        },
    ];

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            deps_mut.api,
            mock_info(owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                operators: None,
                modules: None,
                primitive_contract: None,
            },
        )
        .unwrap();

    let info = mock_info(owner, &[]);

    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let splitter = Splitter {
        recipients: recipient,
        locked: false,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Works

    let msg = ExecuteMsg::AddRecipient {
        recipient: AddressWeight {
            recipient: Recipient::from_string(String::from("addr4")),
            weight: Uint128::new(100),
        },
    };

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    assert_eq!(
        Response::default().add_attributes(vec![attr("action", "added_recipient")]),
        res
    );
    // Add a duplicate user
    let msg = ExecuteMsg::AddRecipient {
        recipient: AddressWeight {
            recipient: Recipient::from_string(String::from("addr4")),
            weight: Uint128::new(100),
        },
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res, ContractError::DuplicateRecipient {});

    let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
    let expected_splitter = Splitter {
        recipients: vec![
            AddressWeight {
                recipient: Recipient::from_string(String::from("addr1")),
                weight: Uint128::new(40),
            },
            AddressWeight {
                recipient: Recipient::from_string(String::from("addr2")),
                weight: Uint128::new(60),
            },
            AddressWeight {
                recipient: Recipient::from_string(String::from("addr3")),
                weight: Uint128::new(50),
            },
            AddressWeight {
                recipient: Recipient::from_string(String::from("addr4")),
                weight: Uint128::new(100),
            },
        ],
        locked: false,
    };
    assert_eq!(expected_splitter, splitter);

    // check result
    let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
    assert_eq!(
        splitter.recipients[3],
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr4")),
            weight: Uint128::new(100),
        }
    );
    assert_eq!(splitter.recipients.len(), 4);
}

#[test]
fn test_execute_add_recipient_duplicate_recipient() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let owner = "creator";

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr1")),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr2")),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr3")),
            weight: Uint128::new(50),
        },
    ];

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            deps_mut.api,
            mock_info(owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                operators: None,
                modules: None,
                primitive_contract: None,
            },
        )
        .unwrap();

    let info = mock_info(owner, &[]);

    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let splitter = Splitter {
        recipients: recipient,
        locked: false,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Works

    let msg = ExecuteMsg::AddRecipient {
        recipient: AddressWeight {
            recipient: Recipient::from_string(String::from("addr4")),
            weight: Uint128::new(100),
        },
    };

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    assert_eq!(
        Response::default().add_attributes(vec![attr("action", "added_recipient")]),
        res
    );
    // Add a duplicate user
    let msg = ExecuteMsg::AddRecipient {
        recipient: AddressWeight {
            recipient: Recipient::from_string(String::from("addr4")),
            weight: Uint128::new(100),
        },
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res, ContractError::DuplicateRecipient {});
}
#[test]
fn test_execute_add_recipient_invalid_weight() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let owner = "creator";

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr1")),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr2")),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr3")),
            weight: Uint128::new(50),
        },
    ];

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            deps_mut.api,
            mock_info(owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                operators: None,
                modules: None,
                primitive_contract: None,
            },
        )
        .unwrap();

    let info = mock_info(owner, &[]);

    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let splitter = Splitter {
        recipients: recipient,
        locked: false,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Invalid weight

    let msg = ExecuteMsg::AddRecipient {
        recipient: AddressWeight {
            recipient: Recipient::from_string(String::from("addr4")),
            weight: Uint128::zero(),
        },
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(ContractError::InvalidWeight {}, res);
}

#[test]
fn test_execute_add_recipient_unauthorized() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let owner = "creator";

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr1")),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr2")),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr3")),
            weight: Uint128::new(50),
        },
    ];
    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient,
    };

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            deps_mut.api,
            mock_info(owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                operators: None,
                modules: None,
                primitive_contract: None,
            },
        )
        .unwrap();
    let info = mock_info("incorrect_owner", &[]);
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_execute_update_recipients() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let owner = "creator";

    let splitter = Splitter {
        recipients: vec![],
        locked: false,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            deps_mut.api,
            mock_info(owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                operators: None,
                modules: None,
                primitive_contract: None,
            },
        )
        .unwrap();

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr1")),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr2")),
            weight: Uint128::new(60),
        },
    ];
    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let info = mock_info(owner, &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        Response::default().add_attributes(vec![attr("action", "update_recipients")]),
        res
    );

    //check result
    let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
    assert_eq!(splitter.recipients, recipient);
}

#[test]
fn test_execute_update_recipients_invalid_weight() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let owner = "creator";

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr1")),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr2")),
            weight: Uint128::zero(),
        },
    ];
    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient,
    };

    let splitter = Splitter {
        recipients: vec![],
        locked: false,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            deps_mut.api,
            mock_info(owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                operators: None,
                modules: None,
                primitive_contract: None,
            },
        )
        .unwrap();

    // Invalid weight

    let info = mock_info(owner, &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res, ContractError::InvalidWeight {});
}

#[test]
fn test_execute_update_recipients_unauthorized() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let owner = "creator";

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr1")),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr2")),
            weight: Uint128::zero(),
        },
    ];
    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient,
    };

    let splitter = Splitter {
        recipients: vec![],
        locked: false,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            deps_mut.api,
            mock_info(owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                operators: None,
                modules: None,
                primitive_contract: None,
            },
        )
        .unwrap();

    // Unauthorized

    let info = mock_info("incorrect_owner", &[]);
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_execute_send() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let owner = "creator";

    let recip_address1 = "address1".to_string();
    let recip_weight1 = Uint128::new(10); // Weight of 10

    let recip_address2 = "address2".to_string();
    let recip_percent2 = Uint128::new(20); // Weight of 20

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::Addr(recip_address1.clone()),
            weight: recip_weight1,
        },
        AddressWeight {
            recipient: Recipient::Addr(recip_address2.clone()),
            weight: recip_percent2,
        },
    ];
    let msg = ExecuteMsg::Send {};

    let splitter = Splitter {
        recipients: recipient,
        locked: false,
    };

    let info = mock_info(owner, &[Coin::new(10000_u128, "uluna")]);
    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            deps_mut.api,
            info.clone(),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                operators: None,
                modules: None,
                primitive_contract: None,
            },
        )
        .unwrap();

    SPLITTER.save(deps_mut.storage, &splitter).unwrap();

    let res = execute(deps_mut, env, info, msg).unwrap();

    let expected_res = Response::new()
        .add_submessages(vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: recip_address1,
                amount: vec![Coin::new(3333, "uluna")], // 10000 * (10/30)
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: recip_address2,
                amount: vec![Coin::new(6666, "uluna")], // 10000 * (20/30)
            })),
            SubMsg::new(
                // refunds remainder to sender
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: owner.to_string(),
                    amount: vec![Coin::new(1, "uluna")], // 10000 - (3333+6666)   remainder
                }),
            ),
        ])
        .add_attributes(vec![attr("action", "send"), attr("sender", "creator")]);

    assert_eq!(res, expected_res);
}

#[test]
fn test_query_splitter() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let splitter = Splitter {
        recipients: vec![],
        locked: false,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let query_msg = QueryMsg::GetSplitterConfig {};
    let res = query(deps.as_ref(), env, query_msg).unwrap();
    let val: GetSplitterConfigResponse = from_binary(&res).unwrap();

    assert_eq!(val.config, splitter);
}

#[test]
fn test_query_user_weight() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let user1 = AddressWeight {
        recipient: Recipient::Addr("first".to_string()),
        weight: Uint128::new(5),
    };
    let user2 = AddressWeight {
        recipient: Recipient::Addr("second".to_string()),
        weight: Uint128::new(10),
    };
    let splitter = Splitter {
        recipients: vec![user1, user2],
        locked: false,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let query_msg = QueryMsg::GetUserWeight {
        user: Recipient::Addr("second".to_string()),
    };
    let res = query(deps.as_ref(), env, query_msg).unwrap();
    let val: GetUserWeightResponse = from_binary(&res).unwrap();

    assert_eq!(val.weight, Uint128::new(10));
    assert_eq!(val.total_weight, Uint128::new(15));
}

#[test]
fn test_execute_send_error() {
    // Send more than 5 coins
    let mut deps = mock_dependencies();
    let env = mock_env();

    let sender_funds_amount = 10000u128;
    let owner = "creator";

    let recip_address1 = "address1".to_string();
    let recip_weight1 = Uint128::new(10); // Weight of 10

    let recip_address2 = "address2".to_string();
    let recip_weight2 = Uint128::new(20); // Weight of 20

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::Addr(recip_address1),
            weight: recip_weight1,
        },
        AddressWeight {
            recipient: Recipient::Addr(recip_address2),
            weight: recip_weight2,
        },
    ];
    let msg = ExecuteMsg::Send {};

    let info = mock_info(
        owner,
        &vec![
            Coin::new(sender_funds_amount, "uluna"),
            Coin::new(sender_funds_amount, "uluna"),
            Coin::new(sender_funds_amount, "uluna"),
            Coin::new(sender_funds_amount, "uluna"),
            Coin::new(sender_funds_amount, "uluna"),
            Coin::new(sender_funds_amount, "uluna"),
        ],
    );
    let splitter = Splitter {
        recipients: recipient.clone(),
        locked: false,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();

    let expected_res = ContractError::ExceedsMaxAllowedCoins {};

    assert_eq!(res, expected_res);

    // Send 0 coins
    let info = mock_info(owner, &[]);
    let splitter = Splitter {
        recipients: recipient,
        locked: false,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();

    let expected_res = ContractError::InvalidFunds {
        msg: "Require at least one coin to be sent".to_string(),
    };

    assert_eq!(res, expected_res);
}
