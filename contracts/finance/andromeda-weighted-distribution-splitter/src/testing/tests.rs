use crate::testing::mock_querier::mock_dependencies_custom;
use andromeda_finance::weighted_splitter::{AddressWeight, ExecuteMsg, InstantiateMsg, Splitter};
use andromeda_std::{
    ado_base::InstantiateMsg as BaseInstantiateMsg,
    ado_contract::ADOContract,
    amp::{recipient::Recipient, AndrAddr},
    common::{expiration::Expiry, Milliseconds},
    error::ContractError,
    testing::mock_querier::MOCK_KERNEL_CONTRACT,
};
use cosmwasm_std::{
    attr,
    testing::{message_info, mock_dependencies, mock_env},
    Addr, BankMsg, Coin, CosmosMsg, QuerierWrapper, Response, SubMsg, Uint128,
};

use crate::{
    contract::{execute, instantiate},
    state::SPLITTER,
};
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const OWNER: &str = "cosmwasm1fsgzj6t7udv8zhf6zj32mkqhcjcpv52yph5qsdcl0qt94jgdckqs2g053y";

fn init(deps: &mut TestDeps) -> Response {
    let some_address = deps.api.addr_make("some_address");
    let mock_recipient: Vec<AddressWeight> = vec![AddressWeight {
        recipient: Recipient::from_string(some_address.to_string()),
        weight: Uint128::new(100),
    }];
    let owner = Addr::unchecked(OWNER);
    let msg = InstantiateMsg {
        owner: Some(OWNER.to_owned()),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        recipients: mock_recipient,
        lock_time: Some(Expiry::FromNow(Milliseconds(86400000))),
        default_recipient: None,
    };

    let info = message_info(&owner, &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap()
}
#[test]
fn test_update_app_contract() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = message_info(&Addr::unchecked(OWNER), &[]);
    let recipient1 = deps.api.addr_make("recipient1");
    let recipient2 = deps.api.addr_make("recipient2");
    let msg = InstantiateMsg {
        recipients: vec![
            AddressWeight {
                recipient: Recipient::new(recipient1, None),
                weight: Uint128::new(50),
            },
            AddressWeight {
                recipient: Recipient::new(recipient2, None),
                weight: Uint128::new(50),
            },
        ],
        lock_time: None,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        default_recipient: None,
    };

    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let app_contract = deps.api.addr_make("app_contract");
    let msg = ExecuteMsg::UpdateAppContract {
        address: app_contract.to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "update_app_contract")
            .add_attribute("address", app_contract.to_string()),
        res
    );
}

// #[test]
// fn test_update_app_contract_invalid_recipient() {
//     let mut deps = mock_dependencies_custom(&[]);

//     let modules: Vec<Module> = vec![Module {
//         name: Some("ks".to_string()),
//         address: AndrAddr::from_string("z".to_string()),
//         is_mutable: false,
//     }];

//     let info = message_info("app_contract", &[]);
//     let msg = InstantiateMsg {
//
//         recipients: vec![AddressWeight {
//             recipient: Recipient::new(MOCK_RECIPIENT1, None),
//             weight: Uint128::new(100),
//         }],
//         lock_time: Some(100_000),
//         kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
//         owner: None,
//     };

//     let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//     let msg = ExecuteMsg::UpdateAppContract {
//         address: "app_contract".to_string(),
//     };

//     let res = execute(deps.as_mut(), mock_env(), info, msg);

//     assert_eq!(ContractError::InvalidAddress {}, res.unwrap_err());
// }

#[test]
fn test_instantiate() {
    let mut deps: cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        cosmwasm_std::testing::MockQuerier,
    > = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked(OWNER), &[]);
    let recipient1 = deps.api.addr_make("recipient1");
    let msg = InstantiateMsg {
        recipients: vec![AddressWeight {
            recipient: Recipient::from_string(recipient1.to_string()),
            weight: Uint128::new(1),
        }],

        lock_time: None,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        default_recipient: None,
    };
    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_execute_update_lock() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let current_time = env.block.time.seconds();
    // 2 days in milliseconds
    let lock_time = Milliseconds(172800000);

    let owner = deps.api.addr_make("owner");

    // Start off with an expiration that's behind current time (expired)
    let splitter = Splitter {
        recipients: vec![],
        lock: Milliseconds(current_time - 1),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let msg = ExecuteMsg::UpdateLock {
        lock_time: Expiry::FromNow(lock_time),
    };
    ADOContract::default()
        .instantiate(
            &mut deps.storage,
            mock_env(),
            &deps.api,
            &QuerierWrapper::new(&deps.querier),
            message_info(&owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    let info = message_info(&owner, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let new_lock = lock_time
        .plus_seconds(current_time)
        .plus_milliseconds(Milliseconds(879));
    assert_eq!(
        Response::default().add_attributes(vec![
            attr("action", "update_lock"),
            attr("locked", new_lock.to_string())
        ]),
        res
    );

    //check result
    let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
    assert!(!splitter.lock.is_expired(&env.block));
}

#[test]
fn test_execute_update_lock_too_short() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let current_time = env.block.time.seconds();
    let lock_time = Milliseconds(1);

    let owner = deps.api.addr_make("owner");

    // Start off with an expiration that's behind current time (expired)
    let splitter = Splitter {
        recipients: vec![],
        lock: Milliseconds(current_time - 1),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let msg = ExecuteMsg::UpdateLock {
        lock_time: Expiry::FromNow(lock_time),
    };
    ADOContract::default()
        .instantiate(
            &mut deps.storage,
            mock_env(),
            &deps.api,
            &QuerierWrapper::new(&deps.querier),
            message_info(&owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    let info = message_info(&owner, &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(ContractError::LockTimeTooShort {}, res);
}

#[test]
fn test_execute_update_lock_too_long() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let current_time = env.block.time.seconds();
    // 25 months
    let lock_time = Milliseconds(65_743_650_000);

    let owner = deps.api.addr_make("owner");

    // Start off with an expiration that's behind current time (expired)
    let splitter = Splitter {
        recipients: vec![],
        lock: Milliseconds(current_time - 1),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let msg = ExecuteMsg::UpdateLock {
        lock_time: Expiry::FromNow(lock_time),
    };
    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    let info = message_info(&owner, &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(ContractError::LockTimeTooLong {}, res);
}

#[test]
fn test_execute_update_lock_already_locked() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let current_time = env.block.time.seconds();

    let lock_time = Milliseconds(172800000);

    let owner = deps.api.addr_make("owner");

    // Start off with an expiration that's ahead current time (unexpired)
    let splitter = Splitter {
        recipients: vec![],
        lock: Milliseconds::default().plus_seconds(current_time + 10_000),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let msg = ExecuteMsg::UpdateLock {
        lock_time: Expiry::FromNow(lock_time),
    };
    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    let info = message_info(&owner, &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(ContractError::ContractLocked { msg: None }, res);
}

#[test]
fn test_execute_update_lock_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let current_time = env.block.time.seconds();
    let lock_time = Milliseconds(100_000);

    let owner = deps.api.addr_make("owner");
    let new_lock = Milliseconds(current_time - 1);

    let splitter = Splitter {
        recipients: vec![],
        lock: new_lock,
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let msg = ExecuteMsg::UpdateLock {
        lock_time: Expiry::FromNow(lock_time),
    };
    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    let incorrect_owner = deps.api.addr_make("incorrect_owner");
    let info = message_info(&incorrect_owner, &[]);
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_execute_remove_recipient() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let owner = deps.api.addr_make("owner");

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
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    let info = message_info(&owner, &[]);

    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let splitter = Splitter {
        recipients: recipient,
        lock: Milliseconds::default(),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::RemoveRecipient {
        recipient: AndrAddr::from_string("addr1"),
    };
    // Try removing a user that isn't in the list
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

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
        lock: Milliseconds::default(),
        default_recipient: None,
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
fn test_execute_remove_recipient_not_on_list() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let addr1 = deps.api.addr_make("addr1");
    let addr2 = deps.api.addr_make("addr2");
    let addr3 = deps.api.addr_make("addr3");

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(addr1.to_string()),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr2.to_string()),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr3.to_string()),
            weight: Uint128::new(50),
        },
    ];

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&Addr::unchecked(OWNER), &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    let info = message_info(&Addr::unchecked(OWNER), &[]);

    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let splitter = Splitter {
        recipients: recipient,
        lock: Milliseconds::default(),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Try removing a user that isn't in the list
    let msg = ExecuteMsg::RemoveRecipient {
        recipient: AndrAddr::from_string("addr10"),
    };

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::UserNotFound {});
}

#[test]
fn test_execute_remove_recipient_contract_locked() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let addr1 = deps.api.addr_make("addr1");
    let addr2 = deps.api.addr_make("addr2");
    let addr3 = deps.api.addr_make("addr3");

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(addr1.to_string()),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr2.to_string()),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr3.to_string()),
            weight: Uint128::new(50),
        },
    ];

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&Addr::unchecked(OWNER), &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    let info = message_info(&Addr::unchecked(OWNER), &[]);

    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let splitter = Splitter {
        recipients: recipient.clone(),
        lock: Milliseconds::default(),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let current_time = env.block.time.seconds();
    let splitter = Splitter {
        recipients: recipient,
        lock: Milliseconds::default().plus_seconds(current_time + 10_000),
        default_recipient: None,
    };
    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let msg = ExecuteMsg::RemoveRecipient {
        recipient: AndrAddr::from_string("addr1"),
    };

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::ContractLocked { msg: None });
}

#[test]
fn test_execute_remove_recipient_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let addr1 = deps.api.addr_make("addr1");
    let addr2 = deps.api.addr_make("addr2");
    let addr3 = deps.api.addr_make("addr3");

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(addr1.to_string()),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr2.to_string()),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr3.to_string()),
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
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&Addr::unchecked(OWNER), &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    let incorrect_owner = deps.api.addr_make("incorrect_owner");
    let info = message_info(&Addr::unchecked(incorrect_owner), &[]);
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_update_recipient_weight() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let addr1 = deps.api.addr_make("addr1");
    let addr2 = deps.api.addr_make("addr2");
    let addr3 = deps.api.addr_make("addr3");

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(addr1.to_string()),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr2.to_string()),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr3.to_string()),
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
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&Addr::unchecked(OWNER), &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    let incorrect_owner = deps.api.addr_make("incorrect_owner");
    let info = message_info(&Addr::unchecked(incorrect_owner), &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

    let info = message_info(&Addr::unchecked(OWNER), &[]);

    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let splitter = Splitter {
        recipients: recipient.clone(),
        lock: Milliseconds::default(),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Works
    let splitter = Splitter {
        recipients: recipient,
        lock: Milliseconds::default(),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();
    let msg = ExecuteMsg::UpdateRecipientWeight {
        recipient: AddressWeight {
            recipient: Recipient::from_string(addr1.to_string()),
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
                recipient: Recipient::from_string(addr1.to_string()),
                weight: Uint128::new(100),
            },
            AddressWeight {
                recipient: Recipient::from_string(addr2.to_string()),
                weight: Uint128::new(60),
            },
            AddressWeight {
                recipient: Recipient::from_string(addr3.to_string()),
                weight: Uint128::new(50),
            },
        ],
        lock: Milliseconds::default(),
        default_recipient: None,
    };
    assert_eq!(expected_splitter, splitter);
}

#[test]
fn test_update_recipient_weight_locked_contract() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let addr1 = deps.api.addr_make("addr1");
    let addr2 = deps.api.addr_make("addr2");
    let addr3 = deps.api.addr_make("addr3");

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(addr1.to_string()),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr2.to_string()),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr3.to_string()),
            weight: Uint128::new(50),
        },
    ];

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&Addr::unchecked(OWNER), &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    let info = message_info(&Addr::unchecked(OWNER), &[]);

    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let current_time = env.block.time.seconds();
    let splitter = Splitter {
        recipients: recipient.clone(),
        lock: Milliseconds(current_time - 1),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Locked contract
    let splitter = Splitter {
        recipients: recipient,
        lock: Milliseconds::default().plus_seconds(current_time + 1),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();
    let msg = ExecuteMsg::UpdateRecipientWeight {
        recipient: AddressWeight {
            recipient: Recipient::from_string(String::from("addr1")),
            weight: Uint128::new(100),
        },
    };
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::ContractLocked { msg: None });
}

#[test]
fn test_update_recipient_weight_user_not_found() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let addr1 = deps.api.addr_make("addr1");
    let addr2 = deps.api.addr_make("addr2");
    let addr3 = deps.api.addr_make("addr3");

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(addr1.to_string()),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr2.to_string()),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr3.to_string()),
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
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&Addr::unchecked(OWNER), &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    let incorrect_owner = deps.api.addr_make("incorrect_owner");
    let info = message_info(&Addr::unchecked(incorrect_owner), &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

    let info = message_info(&Addr::unchecked(OWNER), &[]);

    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let splitter = Splitter {
        recipients: recipient,
        lock: Milliseconds::default(),
        default_recipient: None,
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
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let addr1 = deps.api.addr_make("addr1");
    let addr2 = deps.api.addr_make("addr2");
    let addr3 = deps.api.addr_make("addr3");

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(addr1.to_string()),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr2.to_string()),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr3.to_string()),
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
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&Addr::unchecked(OWNER), &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    let incorrect_owner = deps.api.addr_make("incorrect_owner");
    let info = message_info(&Addr::unchecked(incorrect_owner), &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

    let info = message_info(&Addr::unchecked(OWNER), &[]);

    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let splitter = Splitter {
        recipients: recipient,
        lock: Milliseconds::default(),
        default_recipient: None,
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
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let addr1 = deps.api.addr_make("addr1");
    let addr2 = deps.api.addr_make("addr2");
    let addr3 = deps.api.addr_make("addr3");
    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(addr1.to_string()),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr2.to_string()),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr3.to_string()),
            weight: Uint128::new(50),
        },
    ];

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&Addr::unchecked(OWNER), &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),
                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    let info = message_info(&Addr::unchecked(OWNER), &[]);

    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let splitter = Splitter {
        recipients: recipient,
        lock: Milliseconds::default(),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Works

    let addr4 = deps.api.addr_make("addr4");
    let msg = ExecuteMsg::AddRecipient {
        recipient: AddressWeight {
            recipient: Recipient::from_string(addr4.to_string()),
            weight: Uint128::new(100),
        },
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        Response::default().add_attributes(vec![attr("action", "added_recipient")]),
        res
    );

    let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
    let expected_splitter = Splitter {
        recipients: vec![
            AddressWeight {
                recipient: Recipient::from_string(addr1.to_string()),
                weight: Uint128::new(40),
            },
            AddressWeight {
                recipient: Recipient::from_string(addr2.to_string()),
                weight: Uint128::new(60),
            },
            AddressWeight {
                recipient: Recipient::from_string(addr3.to_string()),
                weight: Uint128::new(50),
            },
            AddressWeight {
                recipient: Recipient::from_string(addr4.to_string()),
                weight: Uint128::new(100),
            },
        ],
        lock: Milliseconds::default(),
        default_recipient: None,
    };
    assert_eq!(expected_splitter, splitter);

    // check result
    let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
    assert_eq!(
        splitter.recipients[3],
        AddressWeight {
            recipient: Recipient::from_string(addr4.to_string()),
            weight: Uint128::new(100),
        }
    );
    assert_eq!(splitter.recipients.len(), 4);
}

#[test]
fn test_execute_add_recipient_duplicate_recipient() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let addr1 = deps.api.addr_make("addr1");
    let addr2 = deps.api.addr_make("addr2");
    let addr3 = deps.api.addr_make("addr3");
    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(addr1.to_string()),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr2.to_string()),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr3.to_string()),
            weight: Uint128::new(50),
        },
    ];

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&Addr::unchecked(OWNER), &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    let info = message_info(&Addr::unchecked(OWNER), &[]);

    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let splitter = Splitter {
        recipients: recipient,
        lock: Milliseconds::default(),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Works

    let addr4 = deps.api.addr_make("addr4");
    let msg = ExecuteMsg::AddRecipient {
        recipient: AddressWeight {
            recipient: Recipient::from_string(addr4.to_string()),
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
            recipient: Recipient::from_string(addr4.to_string()),
            weight: Uint128::new(100),
        },
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res, ContractError::DuplicateRecipient {});
}
#[test]
fn test_execute_add_recipient_invalid_weight() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let addr1 = deps.api.addr_make("addr1");
    let addr2 = deps.api.addr_make("addr2");
    let addr3 = deps.api.addr_make("addr3");

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(addr1.to_string()),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr2.to_string()),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr3.to_string()),
            weight: Uint128::new(50),
        },
    ];

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&Addr::unchecked(OWNER), &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    let info = message_info(&Addr::unchecked(OWNER), &[]);

    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let splitter = Splitter {
        recipients: recipient,
        lock: Milliseconds::default(),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Invalid weight
    let addr4 = deps.api.addr_make("addr4");
    let msg = ExecuteMsg::AddRecipient {
        recipient: AddressWeight {
            recipient: Recipient::from_string(addr4.to_string()),
            weight: Uint128::zero(),
        },
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(ContractError::InvalidWeight {}, res);
}

#[test]
fn test_execute_add_recipient_locked_contract() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let addr1 = deps.api.addr_make("addr1");
    let addr2 = deps.api.addr_make("addr2");
    let addr3 = deps.api.addr_make("addr3");

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(addr1.to_string()),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr2.to_string()),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr3.to_string()),
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
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&Addr::unchecked(OWNER), &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();
    let info = message_info(&Addr::unchecked(OWNER), &[]);
    let current_time = env.block.time.seconds();
    let splitter = Splitter {
        recipients: recipient,
        lock: Milliseconds::default().plus_seconds(current_time + 1),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(
        ContractError::ContractLocked { msg: None },
        res.unwrap_err()
    );
}

#[test]
fn test_execute_add_recipient_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let addr1 = deps.api.addr_make("addr1");
    let addr2 = deps.api.addr_make("addr2");
    let addr3 = deps.api.addr_make("addr3");

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(addr1.to_string()),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr2.to_string()),
            weight: Uint128::new(60),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr3.to_string()),
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
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&Addr::unchecked(OWNER), &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();
    let incorrect_owner = deps.api.addr_make("incorrect_owner");
    let info = message_info(&Addr::unchecked(incorrect_owner), &[]);
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_execute_update_recipients() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let addr1 = deps.api.addr_make("addr1");
    let addr2 = deps.api.addr_make("addr2");

    let splitter = Splitter {
        recipients: vec![],
        lock: Milliseconds::default(),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&Addr::unchecked(OWNER), &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(addr1.to_string()),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(addr2.to_string()),
            weight: Uint128::new(60),
        },
    ];
    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };
    let info = message_info(&Addr::unchecked(OWNER), &[]);
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
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let owner = deps.api.addr_make("owner");

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
        lock: Milliseconds::default(),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    // Invalid weight

    let info = message_info(&owner, &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res, ContractError::InvalidWeight {});
}

#[test]
fn test_execute_update_recipients_contract_locked() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let owner = deps.api.addr_make("owner");

    let recipient = vec![
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr1")),
            weight: Uint128::new(40),
        },
        AddressWeight {
            recipient: Recipient::from_string(String::from("addr2")),
            weight: Uint128::new(100),
        },
    ];
    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient,
    };

    let current_time = env.block.time.seconds();

    let splitter = Splitter {
        recipients: vec![],
        lock: Milliseconds::default().plus_seconds(current_time + 10),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    // Invalid weight

    let info = message_info(&owner, &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res, ContractError::ContractLocked { msg: None });
}

#[test]
fn test_execute_update_recipients_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let owner = deps.api.addr_make("owner");

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
        lock: Milliseconds::default(),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let deps_mut = deps.as_mut();
    ADOContract::default()
        .instantiate(
            deps_mut.storage,
            mock_env(),
            deps_mut.api,
            &deps_mut.querier,
            message_info(&owner, &[]),
            BaseInstantiateMsg {
                ado_type: "splitter".to_string(),
                ado_version: CONTRACT_VERSION.to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();

    // Unauthorized

    let incorrect_owner = deps.api.addr_make("incorrect_owner");
    let info = message_info(&incorrect_owner, &[]);
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_execute_send() {
    let mut deps: cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        crate::testing::mock_querier::WasmMockQuerier,
    > = mock_dependencies_custom(&[]);
    let env = mock_env();
    let _res: Response = init(&mut deps);

    let sender_funds_amount = 10000u128;

    let info = message_info(
        &Addr::unchecked(OWNER),
        &[Coin::new(sender_funds_amount, "uluna")],
    );
    let addr1 = deps.api.addr_make("addr1");
    let addr2 = deps.api.addr_make("addr2");
    let addr3 = deps.api.addr_make("addr3");

    let recip_weight1 = 10; // 10%
    let recip_weight2 = 20; // 20%
    let recip_weight3 = 50; // 50%

    let recip1 = Recipient::from_string(addr1.to_string());
    let recip2 = Recipient::from_string(addr2.to_string());
    let recip3 = Recipient::from_string(addr3.to_string());

    let config_recipient = vec![AddressWeight {
        recipient: recip3.clone(),
        weight: Uint128::new(recip_weight3),
    }];

    let recipient = vec![
        AddressWeight {
            recipient: recip1.clone(),
            weight: Uint128::new(recip_weight1),
        },
        AddressWeight {
            recipient: recip2.clone(),
            weight: Uint128::new(recip_weight2),
        },
    ];
    let msg = ExecuteMsg::Send { config: None };

    let splitter = Splitter {
        recipients: recipient,
        lock: Milliseconds::default(),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let expected_res = Response::new()
        .add_submessages(vec![
            SubMsg::new(
                // refunds remainder to sender
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: addr1.to_string(),
                    amount: vec![Coin::new(3333_u128, "uluna")],
                }),
            ),
            SubMsg::new(
                // refunds remainder to sender
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: addr2.to_string(),
                    amount: vec![Coin::new(6666_u128, "uluna")],
                }),
            ),
            SubMsg::new(
                // refunds remainder to sender
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: OWNER.to_string(),
                    amount: vec![Coin::new(1_u128, "uluna")],
                }),
            ),
        ])
        .add_attributes(vec![attr("action", "send"), attr("sender", OWNER)]);

    assert_eq!(res, expected_res);

    // Test send with config
    let msg = ExecuteMsg::Send {
        config: Some(config_recipient),
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let expected_res = Response::new()
        .add_submessages(vec![
            SubMsg::new(
                // refunds remainder to sender
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: addr3.to_string(),
                    amount: vec![Coin::new(10000u128, "uluna")],
                }),
            ),
            // amp_msg,
        ])
        .add_attributes(vec![attr("action", "send"), attr("sender", OWNER)]);

    assert_eq!(res, expected_res);
}
use rstest::*;

use super::mock_querier::TestDeps;

#[fixture]
fn locked_splitter() -> (
    cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        crate::testing::mock_querier::WasmMockQuerier,
    >,
    Splitter,
) {
    let mut deps = mock_dependencies_custom(&[]);
    let lock_time = mock_env().block.time.plus_seconds(86400);

    let addr1 = deps.api.addr_make("addr1");
    let addr2 = deps.api.addr_make("addr2");
    // Call instantiate with the recipients
    let msg = InstantiateMsg {
        recipients: vec![
            AddressWeight {
                recipient: Recipient::from_string(addr1.to_string()),
                weight: Uint128::new(40), // 40% weight
            },
            AddressWeight {
                recipient: Recipient::from_string(addr2.to_string()),
                weight: Uint128::new(60), // 60% weight
            },
        ],
        lock_time: Some(Expiry::AtTime(Milliseconds::from_seconds(
            lock_time.seconds(),
        ))),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        default_recipient: None,
    };

    let info = message_info(&Addr::unchecked(OWNER), &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
    (deps, splitter)
}

#[fixture]
fn unlocked_splitter() -> (
    cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        crate::testing::mock_querier::WasmMockQuerier,
    >,
    Splitter,
) {
    let mut deps = mock_dependencies_custom(&[]);
    let addr1 = deps.api.addr_make("addr1");
    let addr2 = deps.api.addr_make("addr2");
    // Call instantiate with the recipients
    let msg = InstantiateMsg {
        recipients: vec![
            AddressWeight {
                recipient: Recipient::from_string(addr1.to_string()),
                weight: Uint128::new(40), // 40% weight
            },
            AddressWeight {
                recipient: Recipient::from_string(addr2.to_string()),
                weight: Uint128::new(60), // 60% weight
            },
        ],
        lock_time: None,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        default_recipient: None,
    };

    let info = message_info(&Addr::unchecked(OWNER), &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
    (deps, splitter)
}

#[rstest]
fn test_send_with_config_locked(
    locked_splitter: (
        cosmwasm_std::OwnedDeps<
            cosmwasm_std::MemoryStorage,
            cosmwasm_std::testing::MockApi,
            crate::testing::mock_querier::WasmMockQuerier,
        >,
        Splitter,
    ),
) {
    let (mut deps, _) = locked_splitter;

    let config = vec![AddressWeight {
        recipient: Recipient::from_string("new_addr".to_string()),
        weight: Uint128::new(100), // 100% weight
    }];

    let msg = ExecuteMsg::Send {
        config: Some(config),
    };

    let info = message_info(&Addr::unchecked(OWNER), &[Coin::new(10000u128, "uluna")]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::ContractLocked {
            msg: Some("Config isn't allowed while the splitter is locked".to_string())
        },
        res.unwrap_err()
    );
}

#[rstest]
fn test_send_with_config_unlocked(
    unlocked_splitter: (
        cosmwasm_std::OwnedDeps<
            cosmwasm_std::MemoryStorage,
            cosmwasm_std::testing::MockApi,
            crate::testing::mock_querier::WasmMockQuerier,
        >,
        Splitter,
    ),
) {
    let (mut deps, _) = unlocked_splitter;

    let new_addr = deps.api.addr_make("new_addr");
    let config = vec![AddressWeight {
        recipient: Recipient::from_string(new_addr.to_string()),
        weight: Uint128::new(100), // 100% weight
    }];

    let msg = ExecuteMsg::Send {
        config: Some(config),
    };

    let info = message_info(&Addr::unchecked(OWNER), &[Coin::new(10000u128, "uluna")]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Verify response contains expected submessages and refund
    assert_eq!(1, res.messages.len());
    assert!(res.attributes.contains(&attr("action", "send")));
}

#[rstest]
fn test_send_without_config_locked(
    locked_splitter: (
        cosmwasm_std::OwnedDeps<
            cosmwasm_std::MemoryStorage,
            cosmwasm_std::testing::MockApi,
            crate::testing::mock_querier::WasmMockQuerier,
        >,
        Splitter,
    ),
) {
    let (mut deps, _) = locked_splitter;

    let msg = ExecuteMsg::Send { config: None };

    let info = message_info(&Addr::unchecked(OWNER), &[Coin::new(10000u128, "uluna")]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Verify response contains expected submessages
    assert_eq!(2, res.messages.len());
    assert!(res.attributes.contains(&attr("action", "send")));
}

#[rstest]
fn test_send_without_config_unlocked(
    unlocked_splitter: (
        cosmwasm_std::OwnedDeps<
            cosmwasm_std::MemoryStorage,
            cosmwasm_std::testing::MockApi,
            crate::testing::mock_querier::WasmMockQuerier,
        >,
        Splitter,
    ),
) {
    let (mut deps, _) = unlocked_splitter;

    let msg = ExecuteMsg::Send { config: None };

    let info = message_info(&Addr::unchecked(OWNER), &[Coin::new(10000u128, "uluna")]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Verify response contains expected submessages
    assert_eq!(2, res.messages.len());
    assert!(res.attributes.contains(&attr("action", "send")));
}
