use andromeda_std::{
    amp::{
        messages::{AMPMsg, AMPPkt},
        recipient::Recipient,
    },
    common::{expiration::Expiry, Milliseconds},
    error::ContractError,
};
use cosmwasm_std::{
    attr, from_json,
    testing::{message_info, mock_env, MOCK_CONTRACT_ADDR},
    to_json_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, Response, SubMsg, Timestamp,
};
pub const OWNER: &str = "cosmwasm1fsgzj6t7udv8zhf6zj32mkqhcjcpv52yph5qsdcl0qt94jgdckqs2g053y";

use super::mock_querier::{TestDeps, MOCK_KERNEL_CONTRACT};

use crate::{
    contract::{execute, instantiate, query},
    state::SPLITTER,
    testing::mock_querier::mock_dependencies_custom,
};
use andromeda_finance::splitter::{
    AddressPercent, ExecuteMsg, GetSplitterConfigResponse, InstantiateMsg, QueryMsg, Splitter,
};

fn init(deps: &mut TestDeps) -> Response {
    let some_recipient = deps.api.addr_make("some_recipient");
    let mock_recipient: Vec<AddressPercent> = vec![AddressPercent {
        recipient: Recipient::from_string(some_recipient),
        percent: Decimal::percent(100),
    }];

    let msg = InstantiateMsg {
        owner: Some(OWNER.to_string()),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        recipients: mock_recipient,
        lock_time: Some(Expiry::FromNow(Milliseconds(86400000))),
        default_recipient: None,
    };

    let info = message_info(&Addr::unchecked(OWNER), &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap()
}

#[test]
fn test_instantiate() {
    let mut deps: cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        crate::testing::mock_querier::WasmMockQuerier,
    > = mock_dependencies_custom(&[]);
    let res = init(&mut deps);
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_different_lock_times() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();

    // Current time
    env.block.time = Timestamp::from_seconds(1724920577);
    // Set a lock time that's less than 1 day in milliseconds
    let mut lock_time = Expiry::FromNow(Milliseconds(60_000));

    let owner = deps.api.addr_make(OWNER);
    let kernel_address = deps.api.addr_make(MOCK_KERNEL_CONTRACT);
    let msg = InstantiateMsg {
        owner: Some(owner.to_string()),
        kernel_address: kernel_address.to_string(),
        recipients: vec![],
        lock_time: Some(lock_time),
        default_recipient: None,
    };

    let info = message_info(&owner, &[]);
    let err = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();

    assert_eq!(err, ContractError::LockTimeTooShort {});

    // Set a lock time that's more than 1 year in milliseconds
    lock_time = Expiry::FromNow(Milliseconds(31_708_800_000));

    let kernel_address = deps.api.addr_make(MOCK_KERNEL_CONTRACT);
    let msg = InstantiateMsg {
        owner: Some(owner.to_string()),
        kernel_address: kernel_address.to_string(),
        recipients: vec![],
        lock_time: Some(lock_time),
        default_recipient: None,
    };

    let err = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();

    assert_eq!(err, ContractError::LockTimeTooLong {});

    // Set a lock time for 20 days in milliseconds
    lock_time = Expiry::FromNow(Milliseconds(1728000000));

    let some_address = deps.api.addr_make("some_address");
    let msg = InstantiateMsg {
        owner: Some(owner.to_string()),
        kernel_address: kernel_address.to_string(),
        recipients: vec![AddressPercent {
            recipient: Recipient::from_string(some_address.to_string()),
            percent: Decimal::percent(100),
        }],
        lock_time: Some(lock_time),
        default_recipient: None,
    };

    let info = message_info(&owner, &[]);
    let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());

    // Here we begin testing Expiry::AtTime
    // Set a lock time that's less than 1 day from current time
    lock_time = Expiry::AtTime(Milliseconds(1724934977000));

    let msg = InstantiateMsg {
        owner: Some(owner.to_string()),
        kernel_address: kernel_address.to_string(),
        recipients: vec![],
        lock_time: Some(lock_time),
        default_recipient: None,
    };

    let info = message_info(&Addr::unchecked(&owner), &[]);
    let err = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(err, ContractError::LockTimeTooShort {});

    // Set a lock time that's more than 1 year from current time in milliseconds
    lock_time = Expiry::AtTime(Milliseconds(1788006977000));

    let msg = InstantiateMsg {
        owner: Some(owner.to_string()),
        kernel_address: kernel_address.to_string(),
        recipients: vec![],
        lock_time: Some(lock_time),
        default_recipient: None,
    };

    let info = message_info(&Addr::unchecked(&owner), &[]);
    let err = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(err, ContractError::LockTimeTooLong {});

    // Set a valid lock time
    lock_time = Expiry::AtTime(Milliseconds(1725021377000));

    let msg = InstantiateMsg {
        owner: Some(owner.to_string()),
        kernel_address: kernel_address.to_string(),
        recipients: vec![AddressPercent {
            recipient: Recipient::from_string(some_address),
            percent: Decimal::percent(100),
        }],
        lock_time: Some(lock_time),
        default_recipient: None,
    };

    let info = message_info(&Addr::unchecked(&owner), &[]);
    let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_execute_update_lock() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(&mut deps);

    let env = mock_env();

    let current_time = env.block.time.seconds();
    // 2 days in milliseconds
    let lock_time = 172800000;

    // Start off with an expiration that's behind current time (expired)
    let splitter = Splitter {
        recipients: vec![],
        lock: Milliseconds::from_seconds(current_time - 1),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let msg = ExecuteMsg::UpdateLock {
        lock_time: Expiry::FromNow(Milliseconds(lock_time)),
    };

    let info = message_info(&Addr::unchecked(OWNER), &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    let new_lock = Milliseconds(lock_time)
        .plus_seconds(current_time)
        .plus_milliseconds(Milliseconds(879));
    let expected_res: Response = Response::new()
        .add_attribute("action", "update_lock")
        .add_attribute("locked", "1571970219879".to_string());
    for attr in expected_res.attributes {
        assert!(
            res.attributes.contains(&attr),
            "Attribute {:?} not found",
            attr,
        );
    }
    for msg in expected_res.messages {
        assert!(res.messages.contains(&msg), "Message {:?} not found", msg,);
    }
    for event in expected_res.events {
        assert!(res.events.contains(&event), "Event {:?} not found", event,);
    }

    //check result
    let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
    assert!(!splitter.lock.is_expired(&env.block));
    assert_eq!(new_lock, splitter.lock);
}

#[test]
fn test_execute_update_recipients() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let _res = init(&mut deps);

    let splitter = Splitter {
        recipients: vec![],
        lock: Milliseconds::from_seconds(0),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let addr1 = deps.api.addr_make("addr1");
    // Duplicate recipients
    let duplicate_recipients = vec![
        AddressPercent {
            recipient: Recipient::from_string(addr1.clone()),
            percent: Decimal::percent(40),
        },
        AddressPercent {
            recipient: Recipient::from_string(addr1),
            percent: Decimal::percent(60),
        },
    ];
    let msg = ExecuteMsg::UpdateRecipients {
        recipients: duplicate_recipients,
    };

    let info = message_info(&Addr::unchecked(OWNER), &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert_eq!(ContractError::DuplicateRecipient {}, res.unwrap_err());

    let addr1 = deps.api.addr_make("addr1");
    let addr2 = deps.api.addr_make("addr2");
    let recipients = vec![
        AddressPercent {
            recipient: Recipient::from_string(addr1.to_string()),
            percent: Decimal::percent(40),
        },
        AddressPercent {
            recipient: Recipient::from_string(addr2.to_string()),
            percent: Decimal::percent(60),
        },
    ];
    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipients.clone(),
    };

    let incorrect_owner = deps.api.addr_make("incorrect_owner");
    let info = message_info(&incorrect_owner, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

    let info = message_info(&Addr::unchecked(OWNER), &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    let expected_res: Response = Response::new().add_attribute("action", "update_recipients");
    for attr in expected_res.attributes {
        assert!(
            res.attributes.contains(&attr),
            "Attribute {:?} not found",
            attr,
        );
    }
    for msg in expected_res.messages {
        assert!(res.messages.contains(&msg), "Message {:?} not found", msg,);
    }
    for event in expected_res.events {
        assert!(res.events.contains(&event), "Event {:?} not found", event,);
    }

    //check result
    let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
    assert_eq!(splitter.recipients, recipients);
}

#[test]
fn test_execute_send() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let _res: Response = init(&mut deps);

    let sender_funds_amount = 10000u128;

    let info = message_info(
        &Addr::unchecked(OWNER),
        &[Coin::new(sender_funds_amount, "uluna")],
    );

    let recip_address1 = deps.api.addr_make("address1");
    let recip_percent1 = 10; // 10%

    let recip_address2 = deps.api.addr_make("address2");
    let recip_percent2 = 20; // 20%

    let recip_address3 = deps.api.addr_make("address3");
    let recip_percent3 = 50; // 50%

    let recip1 = Recipient::from_string(recip_address1);
    let recip2 = Recipient::from_string(recip_address2);
    let recip3 = Recipient::from_string(recip_address3.clone());

    let config_recipient = vec![AddressPercent {
        recipient: recip3.clone(),
        percent: Decimal::percent(recip_percent3),
    }];

    let recipient = vec![
        AddressPercent {
            recipient: recip1.clone(),
            percent: Decimal::percent(recip_percent1),
        },
        AddressPercent {
            recipient: recip2.clone(),
            percent: Decimal::percent(recip_percent2),
        },
    ];
    let msg = ExecuteMsg::Send { config: None };

    let amp_msg_1 = recip1
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(1000u128, "uluna")]))
        .unwrap();
    let amp_msg_2 = recip2
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(2000u128, "uluna")]))
        .unwrap();
    let amp_pkt = AMPPkt::new(
        MOCK_CONTRACT_ADDR.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        vec![amp_msg_1, amp_msg_2],
    );
    let amp_msg = amp_pkt
        .to_sub_msg(
            Addr::unchecked(MOCK_KERNEL_CONTRACT),
            Some(vec![
                Coin::new(1000u128, "uluna"),
                Coin::new(2000u128, "uluna"),
            ]),
            1,
        )
        .unwrap();

    let splitter = Splitter {
        recipients: recipient.clone(),
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
                    to_address: OWNER.to_string(),
                    amount: vec![Coin::new(7000u128, "uluna")], // 10000 * 0.7   remainder
                }),
            ),
            amp_msg,
        ])
        .add_attributes(vec![
            attr("action", "send"),
            attr("sender", OWNER.to_string()),
        ]);

    for attr in expected_res.attributes {
        assert!(
            res.attributes.contains(&attr),
            "Attribute {:?} not found",
            attr,
        );
    }
    for msg in expected_res.messages {
        assert!(res.messages.contains(&msg), "Message {:?} not found", msg,);
    }
    for event in expected_res.events {
        assert!(res.events.contains(&event), "Event {:?} not found", event,);
    }

    // Test send with config
    let msg = ExecuteMsg::Send {
        config: Some(config_recipient),
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    let amp_msg_1 = recip3
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(5000u128, "uluna")]))
        .unwrap();

    let amp_pkt = AMPPkt::new(
        MOCK_CONTRACT_ADDR.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        vec![amp_msg_1],
    );

    let amp_msg = amp_pkt
        .to_sub_msg(
            Addr::unchecked(MOCK_KERNEL_CONTRACT),
            Some(vec![Coin::new(5000u128, "uluna")]),
            1,
        )
        .unwrap();
    let expected_res = Response::new()
        .add_submessages(vec![
            SubMsg::new(
                // refunds remainder to sender
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: OWNER.to_string(),
                    amount: vec![Coin::new(5000u128, "uluna")], // 10000 * 0.5   remainder
                }),
            ),
            amp_msg.clone(),
        ])
        .add_attributes(vec![
            attr("action", "send"),
            attr("sender", OWNER.to_string()),
        ]);

    for attr in expected_res.attributes {
        assert!(
            res.attributes.contains(&attr),
            "Attribute {:?} not found",
            attr,
        );
    }
    for msg in expected_res.messages {
        assert!(res.messages.contains(&msg), "Message {:?} not found", msg,);
    }
    for event in expected_res.events {
        assert!(res.events.contains(&event), "Event {:?} not found", event,);
    }

    // Test send with default recipient
    let msg = ExecuteMsg::Send { config: None };
    SPLITTER
        .save(
            deps.as_mut().storage,
            &Splitter {
                recipients: recipient,
                lock: Milliseconds::default(),
                default_recipient: Some(recip3.clone()),
            },
        )
        .unwrap();

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    let amp_msg_1 = recip1
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(1000u128, "uluna")]))
        .unwrap();
    let amp_msg_2 = recip2
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(2000u128, "uluna")]))
        .unwrap();
    let amp_pkt = AMPPkt::new(
        MOCK_CONTRACT_ADDR.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        vec![amp_msg_1, amp_msg_2],
    );
    let amp_msg = amp_pkt
        .to_sub_msg(
            Addr::unchecked(MOCK_KERNEL_CONTRACT),
            Some(vec![
                Coin::new(1000u128, "uluna"),
                Coin::new(2000u128, "uluna"),
            ]),
            1,
        )
        .unwrap();

    let expected_res = Response::new()
        .add_submessages(vec![
            SubMsg::new(
                // refunds remainder to sender
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: recip_address3.to_string(),
                    amount: vec![Coin::new(7000u128, "uluna")], // 10000 * 0.7   remainder
                }),
            ),
            amp_msg,
        ])
        .add_attributes(vec![
            attr("action", "send"),
            attr("sender", OWNER.to_string()),
        ]);

    for attr in expected_res.attributes {
        assert!(
            res.attributes.contains(&attr),
            "Attribute {:?} not found",
            attr,
        );
    }
    for msg in expected_res.messages {
        assert!(res.messages.contains(&msg), "Message {:?} not found", msg,);
    }
    for event in expected_res.events {
        assert!(res.events.contains(&event), "Event {:?} not found", event,);
    }
}

#[test]
fn test_execute_send_ado_recipient() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let _res: Response = init(&mut deps);

    let sender_funds_amount = 10000u128;
    let info = message_info(
        &Addr::unchecked(OWNER),
        &[Coin::new(sender_funds_amount, "uluna")],
    );

    let recip_address1 = deps.api.addr_make("address1");
    let recip_percent1 = 10; // 10%

    let recip_address2 = deps.api.addr_make("address2");
    let recip_percent2 = 20; // 20%

    let recip1 = Recipient::from_string(recip_address1);
    let recip2 = Recipient::from_string(recip_address2);

    let recipient = vec![
        AddressPercent {
            recipient: recip1.clone(),
            percent: Decimal::percent(recip_percent1),
        },
        AddressPercent {
            recipient: recip2.clone(),
            percent: Decimal::percent(recip_percent2),
        },
    ];
    let msg = ExecuteMsg::Send { config: None };

    let amp_msg_1 = recip1
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(1000u128, "uluna")]))
        .unwrap();
    let amp_msg_2 = recip2
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(2000u128, "uluna")]))
        .unwrap();
    let amp_pkt = AMPPkt::new(
        MOCK_CONTRACT_ADDR.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        vec![amp_msg_1, amp_msg_2],
    );
    let amp_msg = amp_pkt
        .to_sub_msg(
            Addr::unchecked(MOCK_KERNEL_CONTRACT),
            Some(vec![
                Coin::new(1000u128, "uluna"),
                Coin::new(2000u128, "uluna"),
            ]),
            1,
        )
        .unwrap();

    let splitter = Splitter {
        recipients: recipient,
        lock: Milliseconds::default(),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

    let expected_res = Response::new()
        .add_submessages(vec![
            SubMsg::new(
                // refunds remainder to sender
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: info.sender.to_string(),
                    amount: vec![Coin::new(7000u128, "uluna")], // 10000 * 0.7   remainder
                }),
            ),
            amp_msg,
        ])
        .add_attribute("action", "send")
        .add_attribute("sender", OWNER);

    for attr in expected_res.attributes {
        assert!(
            res.attributes.contains(&attr),
            "Attribute {:?} not found",
            attr,
        );
    }
    for msg in expected_res.messages {
        assert!(res.messages.contains(&msg), "Message {:?} not found", msg,);
    }
    for event in expected_res.events {
        assert!(res.events.contains(&event), "Event {:?} not found", event,);
    }
}

#[test]
fn test_handle_packet_exit_with_error_true() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let _res: Response = init(&mut deps);

    let sender_funds_amount = 0u128;
    let info = message_info(
        &Addr::unchecked(OWNER),
        &[Coin::new(sender_funds_amount, "uluna")],
    );

    let recip_address1 = deps.api.addr_make("address1");
    let recip_percent1 = 10; // 10%

    let recip_percent2 = 20; // 20%

    let recipient = vec![
        AddressPercent {
            recipient: Recipient::from_string(recip_address1.clone()),
            percent: Decimal::percent(recip_percent1),
        },
        AddressPercent {
            recipient: Recipient::from_string(recip_address1.clone()),
            percent: Decimal::percent(recip_percent2),
        },
    ];
    let cosmos_contract = deps.api.addr_make("cosmos2contract");
    let pkt = AMPPkt::new(
        info.clone().sender,
        cosmos_contract,
        vec![AMPMsg::new(
            recip_address1,
            to_json_binary(&ExecuteMsg::Send { config: None }).unwrap(),
            Some(vec![Coin::new(0u128, "uluna")]),
        )],
    );
    let msg = ExecuteMsg::AMPReceive(pkt);

    let splitter = Splitter {
        recipients: recipient,
        lock: Milliseconds::default(),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

    assert_eq!(
        err,
        ContractError::InvalidFunds {
            msg: "Amount must be non-zero".to_string(),
        }
    );
}

#[test]
fn test_query_splitter() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let splitter = Splitter {
        recipients: vec![],
        lock: Milliseconds::default(),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let query_msg = QueryMsg::GetSplitterConfig {};
    let res = query(deps.as_ref(), env, query_msg).unwrap();
    let val: GetSplitterConfigResponse = from_json(res).unwrap();

    assert_eq!(val.config, splitter);
}

#[test]
fn test_execute_send_error() {
    //Executes send with more than 5 tokens [ACK-04]
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let _res: Response = init(&mut deps);

    let sender_funds_amount = 10000u128;
    let owner = "creator";
    let owner = deps.api.addr_make(owner);
    let info = message_info(
        &owner,
        &vec![
            Coin::new(sender_funds_amount, "uluna"),
            Coin::new(sender_funds_amount, "uluna"),
            Coin::new(sender_funds_amount, "uluna"),
            Coin::new(sender_funds_amount, "uluna"),
            Coin::new(sender_funds_amount, "uluna"),
            Coin::new(sender_funds_amount, "uluna"),
        ],
    );

    let recip_address1 = "address1".to_string();
    let recip_percent1 = 10; // 10%

    let recip_address2 = "address2".to_string();
    let recip_percent2 = 20; // 20%

    let recipient = vec![
        AddressPercent {
            recipient: Recipient::from_string(recip_address1),
            percent: Decimal::percent(recip_percent1),
        },
        AddressPercent {
            recipient: Recipient::from_string(recip_address2),
            percent: Decimal::percent(recip_percent2),
        },
    ];
    let msg = ExecuteMsg::Send { config: None };

    let splitter = Splitter {
        recipients: recipient,
        lock: Milliseconds::default(),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();

    let expected_res = ContractError::ExceedsMaxAllowedCoins {};

    assert_eq!(res, expected_res);
}

#[test]
fn test_update_app_contract() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res: Response = init(&mut deps);

    let info = message_info(&Addr::unchecked(OWNER), &[]);

    let app_contract = deps.api.addr_make("app_contract");
    let msg = ExecuteMsg::UpdateAppContract {
        address: app_contract.to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let expected_res: Response = Response::new()
        .add_attribute("action", "update_app_contract")
        .add_attribute("address", app_contract.to_string());
    for attr in expected_res.attributes {
        assert!(
            res.attributes.contains(&attr),
            "Attribute {:?} not found",
            attr,
        );
    }
    for msg in expected_res.messages {
        assert!(res.messages.contains(&msg), "Message {:?} not found", msg,);
    }
    for event in expected_res.events {
        assert!(res.events.contains(&event), "Event {:?} not found", event,);
    }
}

#[test]
fn test_update_app_contract_invalid_recipient() {
    let mut deps: cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        crate::testing::mock_querier::WasmMockQuerier,
    > = mock_dependencies_custom(&[]);
    let _res: Response = init(&mut deps);

    let info = message_info(&Addr::unchecked(OWNER), &[]);

    let msg = ExecuteMsg::UpdateAppContract {
        address: "z".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    // assert_eq!(
    //     ContractError::InvalidComponent {
    //         name: "z".to_string()
    //     },
    //     res.unwrap_err()
    // );
    assert!(res.is_err())
}

use rstest::*;

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
    let splitter = Splitter {
        recipients: vec![
            AddressPercent {
                recipient: Recipient::from_string("addr1".to_string()),
                percent: Decimal::percent(40),
            },
            AddressPercent {
                recipient: Recipient::from_string("addr2".to_string()),
                percent: Decimal::percent(60),
            },
        ],
        lock: Milliseconds::from_seconds(lock_time.seconds()),
        default_recipient: None,
    };
    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();
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
    let splitter = Splitter {
        recipients: vec![
            AddressPercent {
                recipient: Recipient::from_string("addr1".to_string()),
                percent: Decimal::percent(40),
            },
            AddressPercent {
                recipient: Recipient::from_string("addr2".to_string()),
                percent: Decimal::percent(60),
            },
        ],
        lock: Milliseconds::default(),
        default_recipient: None,
    };
    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();
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

    let config = vec![AddressPercent {
        recipient: Recipient::from_string("new_addr".to_string()),
        percent: Decimal::percent(100),
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
    let config = vec![AddressPercent {
        recipient: Recipient::from_string(new_addr.to_string()),
        percent: Decimal::percent(100),
    }];

    let msg = ExecuteMsg::Send {
        config: Some(config),
    };

    let info = message_info(&Addr::unchecked(OWNER), &[Coin::new(10000u128, "uluna")]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Verify response contains expected submessages
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
    assert!(res.attributes.contains(&attr("action", "send")));
}
