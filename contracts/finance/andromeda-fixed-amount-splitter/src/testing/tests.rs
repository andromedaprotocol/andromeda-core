use andromeda_std::{
    amp::{
        messages::{AMPMsg, AMPPkt},
        recipient::Recipient,
    },
    common::{expiration::Expiry, Milliseconds},
    error::ContractError,
};
use cosmwasm_std::{
    attr, coin, coins, from_json,
    testing::{message_info, mock_env, MOCK_CONTRACT_ADDR},
    to_json_binary, Addr, BankMsg, Coin, CosmosMsg, Response, SubMsg,
};
pub const OWNER: &str = "creator";

use super::mock_querier::{TestDeps, MOCK_KERNEL_CONTRACT};

use crate::{
    contract::{execute, instantiate, query},
    state::SPLITTER,
    testing::mock_querier::mock_dependencies_custom,
};
use andromeda_finance::fixed_amount_splitter::{
    AddressAmount, ExecuteMsg, GetSplitterConfigResponse, InstantiateMsg, QueryMsg, Splitter,
};

fn init(deps: &mut TestDeps) -> Response {
    let some_address = deps.api.addr_make("some_address");
    let mock_recipient: Vec<AddressAmount> = vec![AddressAmount {
        recipient: Recipient::from_string(String::from(some_address)),
        coins: coins(1_u128, "uandr"),
    }];
    let owner = deps.api.addr_make(OWNER);
    let msg = InstantiateMsg {
        owner: Some(owner.to_string()),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        recipients: mock_recipient,
        lock_time: Some(Expiry::AtTime(Milliseconds::from_seconds(100_000))),
        default_recipient: None,
    };

    let info = message_info(&owner, &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap()
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);
    let res = init(&mut deps);
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_execute_update_lock() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(&mut deps);

    let env = mock_env();

    let current_time = env.block.time.seconds();
    // 2 days in milliseconds
    let lock_time = Milliseconds(172800000);

    // Start off with an expiration that's behind current time (expired)
    let splitter = Splitter {
        recipients: vec![],
        lock: Milliseconds::from_seconds(current_time - 1),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let msg = ExecuteMsg::UpdateLock {
        lock_time: Expiry::FromNow(lock_time),
    };

    let owner = deps.api.addr_make(OWNER);
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
        AddressAmount {
            recipient: Recipient::from_string(String::from(addr1.clone())),
            coins: coins(1_u128, "uandr"),
        },
        AddressAmount {
            recipient: Recipient::from_string(String::from(addr1.clone())),
            coins: coins(1_u128, "uandr"),
        },
    ];
    let msg = ExecuteMsg::UpdateRecipients {
        recipients: duplicate_recipients,
    };

    let owner = deps.api.addr_make(OWNER);
    let info = message_info(&owner, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert_eq!(ContractError::DuplicateRecipient {}, res.unwrap_err());

    let addr2 = deps.api.addr_make("addr2");
    let recipients = vec![
        AddressAmount {
            recipient: Recipient::from_string(String::from(addr1)),
            coins: coins(1_u128, "uandr"),
        },
        AddressAmount {
            recipient: Recipient::from_string(String::from(addr2)),
            coins: coins(1_u128, "uandr"),
        },
    ];
    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipients.clone(),
    };
    let incorrect_owner = deps.api.addr_make("incorrect_owner");
    let info = message_info(&incorrect_owner, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

    let owner = deps.api.addr_make(OWNER);
    let info = message_info(&owner, &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        Response::default().add_attributes(vec![attr("action", "update_recipients")]),
        res
    );

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

    let owner = deps.api.addr_make(OWNER);
    let info = message_info(
        &owner,
        &[
            Coin::new(sender_funds_amount, "uandr"),
            Coin::new(50_u128, "usdc"),
        ],
    );

    let recip_address1 = deps.api.addr_make("address1").to_string();

    let recip_address2 = deps.api.addr_make("address2").to_string();

    let recip_address3 = deps.api.addr_make("address3").to_string();

    let recip1 = Recipient::from_string(recip_address1);
    let recip2 = Recipient::from_string(recip_address2);
    let recip3 = Recipient::from_string(recip_address3);
    let config_recipient = vec![AddressAmount {
        recipient: recip3.clone(),
        coins: vec![coin(1_u128, "uandr"), coin(30_u128, "usdc")],
    }];
    let recipient = vec![
        AddressAmount {
            recipient: recip1.clone(),
            coins: vec![coin(1_u128, "uandr"), coin(30_u128, "usdc")],
        },
        AddressAmount {
            recipient: recip2.clone(),
            coins: vec![coin(1_u128, "uandr"), coin(20_u128, "usdc")],
        },
    ];
    let msg = ExecuteMsg::Send { config: None };

    let amp_msg_1 = recip1
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(1_u128, "uandr")]))
        .unwrap();
    let amp_msg_2 = recip2
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(1_u128, "uandr")]))
        .unwrap();

    let amp_msg_3 = recip1
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(30_u128, "usdc")]))
        .unwrap();
    let amp_msg_4 = recip2
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(20_u128, "usdc")]))
        .unwrap();

    let amp_pkt = AMPPkt::new(
        MOCK_CONTRACT_ADDR.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        vec![amp_msg_1, amp_msg_2, amp_msg_3, amp_msg_4],
    );
    let amp_msg = amp_pkt
        .to_sub_msg(
            Addr::unchecked(MOCK_KERNEL_CONTRACT),
            Some(vec![
                Coin::new(1_u128, "uandr"),
                Coin::new(1_u128, "uandr"),
                Coin::new(30_u128, "usdc"),
                Coin::new(20_u128, "usdc"),
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

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let expected_res = Response::new()
        .add_submessages(vec![
            SubMsg::new(
                // refunds remainder to sender
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: owner.to_string(),
                    amount: vec![Coin::new(9998_u128, "uandr")],
                }),
            ),
            amp_msg,
        ])
        .add_attributes(vec![
            attr("action", "send"),
            attr("sender", owner.to_string()),
        ]);

    assert_eq!(res, expected_res);

    // Test with config
    let msg = ExecuteMsg::Send {
        config: Some(config_recipient),
    };

    let amp_msg_1 = recip3
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(1_u128, "uandr")]))
        .unwrap();

    let amp_msg_2 = recip3
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(30_u128, "usdc")]))
        .unwrap();

    let amp_pkt = AMPPkt::new(
        MOCK_CONTRACT_ADDR.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        vec![amp_msg_1, amp_msg_2],
    );
    let amp_msg = amp_pkt
        .to_sub_msg(
            Addr::unchecked(MOCK_KERNEL_CONTRACT),
            Some(vec![Coin::new(1_u128, "uandr"), Coin::new(30_u128, "usdc")]),
            1,
        )
        .unwrap();

    let expected_res = Response::new()
        .add_submessages(vec![
            SubMsg::new(
                // refunds remainder to sender
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: owner.to_string(),
                    amount: vec![Coin::new(9999_u128, "uandr")],
                }),
            ),
            SubMsg::new(
                // refunds remainder to sender
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: owner.to_string(),
                    amount: vec![Coin::new(20_u128, "usdc")],
                }),
            ),
            amp_msg,
        ])
        .add_attributes(vec![
            attr("action", "send"),
            attr("sender", owner.to_string()),
        ]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(res, expected_res);
}

#[test]
fn test_execute_send_ado_recipient() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let _res: Response = init(&mut deps);
    let owner = deps.api.addr_make(OWNER);

    let sender_funds_amount = 10_000u128;
    let info = message_info(&owner, &[Coin::new(sender_funds_amount, "uandr")]);

    let recip_address1 = deps.api.addr_make("address1").to_string();

    let recip_address2 = deps.api.addr_make("address2").to_string();

    let recip1 = Recipient::from_string(recip_address1);
    let recip2 = Recipient::from_string(recip_address2);

    let recipient = vec![
        AddressAmount {
            recipient: recip1.clone(),
            coins: coins(1_u128, "uandr"),
        },
        AddressAmount {
            recipient: recip2.clone(),
            coins: coins(1_u128, "uandr"),
        },
    ];
    let msg = ExecuteMsg::Send { config: None };

    let amp_msg_1 = recip1
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(1_u128, "uandr")]))
        .unwrap();
    let amp_msg_2 = recip2
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(1_u128, "uandr")]))
        .unwrap();
    let amp_pkt = AMPPkt::new(
        MOCK_CONTRACT_ADDR.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        vec![amp_msg_1, amp_msg_2],
    );
    let amp_msg = amp_pkt
        .to_sub_msg(
            Addr::unchecked(MOCK_KERNEL_CONTRACT),
            Some(vec![Coin::new(1_u128, "uandr"), Coin::new(1_u128, "uandr")]),
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
                    amount: vec![Coin::new(9_998_u128, "uandr")],
                }),
            ),
            amp_msg,
        ])
        .add_attribute("action", "send")
        .add_attribute("sender", owner.to_string());

    assert_eq!(res, expected_res);
}

#[test]
fn test_handle_packet_exit_with_error_true() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let _res: Response = init(&mut deps);

    let owner = deps.api.addr_make(OWNER);
    let sender_funds_amount = 0u128;
    let info = message_info(&owner, &[Coin::new(sender_funds_amount, "uandr")]);

    let recip_address1 = deps.api.addr_make("address1").to_string();

    let recipient = vec![
        AddressAmount {
            recipient: Recipient::from_string(recip_address1.clone()),
            coins: coins(1_u128, "uandr"),
        },
        AddressAmount {
            recipient: Recipient::from_string(recip_address1.clone()),
            coins: coins(1_u128, "uandr"),
        },
    ];
    let pkt = AMPPkt::new(
        info.clone().sender,
        MOCK_CONTRACT_ADDR.to_string(),
        vec![AMPMsg::new(
            recip_address1,
            to_json_binary(&ExecuteMsg::Send { config: None }).unwrap(),
            Some(vec![Coin::new(0_u128, "uandr")]),
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
    let info = message_info(
        &Addr::unchecked(owner),
        &vec![
            Coin::new(sender_funds_amount, "uandr"),
            Coin::new(sender_funds_amount, "uandr"),
            Coin::new(sender_funds_amount, "uandr"),
            Coin::new(sender_funds_amount, "uandr"),
            Coin::new(sender_funds_amount, "uandr"),
            Coin::new(sender_funds_amount, "uandr"),
        ],
    );

    let recip_address1 = "address1".to_string();

    let recip_address2 = "address2".to_string();

    let recipient = vec![
        AddressAmount {
            recipient: Recipient::from_string(recip_address1),
            coins: coins(1_u128, "uandr"),
        },
        AddressAmount {
            recipient: Recipient::from_string(recip_address2),
            coins: coins(1_u128, "uandr"),
        },
    ];
    let msg = ExecuteMsg::Send { config: None };

    let splitter = Splitter {
        recipients: recipient,
        lock: Milliseconds::default(),
        default_recipient: None,
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();

    let expected_res = ContractError::InvalidFunds {
        msg: "A minimim of 1 and a maximum of 2 coins are allowed".to_string(),
    };

    assert_eq!(res, expected_res);

    // Insufficient funds
    let info = message_info(&Addr::unchecked(owner), &[Coin::new(1_u128, "uandr")]);

    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();

    let expected_res = ContractError::InsufficientFunds {};

    assert_eq!(res, expected_res);
}

#[test]
fn test_update_app_contract() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res: Response = init(&mut deps);

    let owner = deps.api.addr_make(OWNER);
    let info = message_info(&owner, &[]);

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

#[test]
fn test_update_app_contract_invalid_recipient() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res: Response = init(&mut deps);

    let owner = deps.api.addr_make(OWNER);
    let info = message_info(&owner, &[]);

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
            AddressAmount {
                recipient: Recipient::from_string("addr1".to_string()),
                coins: coins(40_u128, "uluna"),
            },
            AddressAmount {
                recipient: Recipient::from_string("addr2".to_string()),
                coins: coins(60_u128, "uluna"),
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
            AddressAmount {
                recipient: Recipient::from_string("addr1".to_string()),
                coins: coins(40_u128, "uluna"),
            },
            AddressAmount {
                recipient: Recipient::from_string("addr2".to_string()),
                coins: coins(60_u128, "uluna"),
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
    let config = vec![AddressAmount {
        recipient: Recipient::from_string("new_addr".to_string()),
        coins: coins(100_u128, "uluna"),
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
    let config = vec![AddressAmount {
        recipient: Recipient::from_string(new_addr.to_string()),
        coins: coins(100_u128, "uluna"),
    }];

    let msg = ExecuteMsg::Send {
        config: Some(config),
    };

    let owner = deps.api.addr_make("owner");
    let info = message_info(&owner, &[Coin::new(10000u128, "uluna")]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Verify response contains expected submessages and refund
    assert_eq!(2, res.messages.len()); // 1 for refund, 1 for transfer
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

    let owner = deps.api.addr_make("owner");
    let info = message_info(&owner, &[Coin::new(10000u128, "uluna")]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Verify response contains expected submessages and refund
    assert_eq!(2, res.messages.len()); // 1 for refund, 1 for transfers
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
    let msg = ExecuteMsg::Send { config: None };
    let (mut deps, _) = unlocked_splitter;
    let owner = deps.api.addr_make("owner");
    let info = message_info(&owner, &[Coin::new(10000u128, "uluna")]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Verify response contains expected submessages and refund
    assert_eq!(2, res.messages.len()); // 1 for refund, 1 for transfers
    assert!(res.attributes.contains(&attr("action", "send")));
}
