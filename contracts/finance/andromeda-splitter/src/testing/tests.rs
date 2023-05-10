use andromeda_std::os::kernel::ExecuteMsg as KernelExecuteMsg;
use andromeda_std::{
    ado_base::modules::Module,
    ado_contract::ADOContract,
    amp::{
        addresses::AndrAddr,
        messages::{AMPMsg, AMPPkt},
        recipient::Recipient,
    },
    common::encode_binary,
    error::ContractError,
    testing::mock_querier::FAKE_VFS_PATH,
};

use cosmwasm_std::Binary;
use cosmwasm_std::{
    attr, coin, coins, from_binary,
    testing::{mock_env, mock_info},
    to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Response, StdError, SubMsg,
    Timestamp, Uint128, WasmMsg,
};
use cw_utils::Expiration;
pub const OWNER: &str = "creator";

use super::mock_querier::MOCK_KERNEL_CONTRACT;

use crate::{
    contract::{execute, instantiate},
    state::SPLITTER,
    testing::mock_querier::mock_dependencies_custom,
};
use andromeda_finance::splitter::{AddressPercent, ExecuteMsg, InstantiateMsg, Splitter};

fn init(deps: DepsMut, modules: Option<Vec<Module>>) -> Response {
    let mock_recipient: Vec<AddressPercent> = vec![AddressPercent {
        recipient: Recipient::from_string(String::from("Some Address")),
        percent: Decimal::percent(100),
    }];
    let msg = InstantiateMsg {
        owner: Some(OWNER.to_owned()),
        modules,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        recipients: mock_recipient,
        lock_time: Some(100_000),
    };

    let info = mock_info("owner", &[]);
    instantiate(deps, mock_env(), info, msg).unwrap()
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);
    let res = init(deps.as_mut(), None);
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_execute_update_lock() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut(), None);

    let env = mock_env();

    let current_time = env.block.time.seconds();
    let lock_time = 100_000;

    // Start off with an expiration that's behind current time (expired)
    let splitter = Splitter {
        recipients: vec![],
        lock: Expiration::AtTime(Timestamp::from_seconds(current_time - 1)),
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let msg = ExecuteMsg::UpdateLock { lock_time };

    let info = mock_info(OWNER, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    let new_lock = Expiration::AtTime(Timestamp::from_seconds(current_time + lock_time));
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
    let _res = init(deps.as_mut(), None);

    let recipient = vec![
        AddressPercent {
            recipient: Recipient::from_string(String::from("addr1")),
            percent: Decimal::percent(40),
        },
        AddressPercent {
            recipient: Recipient::from_string(String::from("addr1")),
            percent: Decimal::percent(60),
        },
    ];
    let msg = ExecuteMsg::UpdateRecipients {
        recipients: recipient.clone(),
    };

    let splitter = Splitter {
        recipients: vec![],
        lock: Expiration::AtTime(Timestamp::from_seconds(0)),
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let info = mock_info("incorrect_owner", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

    let info = mock_info(OWNER, &[]);
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
fn test_execute_send() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let _res: Response = init(deps.as_mut(), None);

    let sender_funds_amount = 10000u128;

    let info = mock_info(OWNER, &[Coin::new(sender_funds_amount, "uluna")]);

    let recip_address1 = "address1".to_string();
    let recip_percent1 = 10; // 10%

    let recip_address2 = "address2".to_string();
    let recip_percent2 = 20; // 20%

    let recipient = vec![
        AddressPercent {
            recipient: Recipient::from_string(recip_address1.clone()),
            percent: Decimal::percent(recip_percent1),
        },
        AddressPercent {
            recipient: Recipient::from_string(recip_address2.clone()),
            percent: Decimal::percent(recip_percent2),
        },
    ];
    let msg = ExecuteMsg::Send {};

    let splitter = Splitter {
        recipients: recipient,
        lock: Expiration::AtTime(Timestamp::from_seconds(0)),
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let deps_mut = deps.as_mut();

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let expected_res = Response::new()
        .add_submessages(vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: recip_address1,
                amount: vec![Coin::new(1000, "uluna")], // 10000 * 0.1
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: recip_address2,
                amount: vec![Coin::new(2000, "uluna")], // 10000 * 0.2
            })),
            SubMsg::new(
                // refunds remainder to sender
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: OWNER.to_string(),
                    amount: vec![Coin::new(7000, "uluna")], // 10000 * 0.7   remainder
                }),
            ),
        ])
        .add_attributes(vec![attr("action", "send"), attr("sender", "creator")]);

    assert_eq!(res, expected_res);
}

#[test]
fn test_execute_send_ado_recipient() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let _res: Response = init(deps.as_mut(), None);

    let sender_funds_amount = 10000u128;
    let info = mock_info(OWNER, &[Coin::new(sender_funds_amount, "uluna")]);

    let recip_address1 = "address1".to_string();
    let recip_percent1 = 10; // 10%

    let recip_address2 = "address2".to_string();
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
    let msg = ExecuteMsg::Send {};

    let splitter = Splitter {
        recipients: recipient,
        lock: Expiration::AtTime(Timestamp::from_seconds(0)),
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let deps_mut = deps.as_mut();

    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

    let pkt = AMPPkt::new(
        info.sender,
        "cosmos2contract",
        vec![
            AMPMsg::new(
                recip_address1,
                Binary::default(),
                Some(vec![Coin::new(1000, "uluna")]),
            ),
            AMPMsg::new(
                recip_address2,
                Binary::default(),
                Some(vec![Coin::new(2000, "uluna")]),
            ),
        ],
    );

    let expected_res = Response::new()
        .add_submessages(vec![
            SubMsg::new(
                // refunds remainder to sender
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: OWNER.to_string(),
                    amount: vec![Coin::new(7000, "uluna")], // 10000 * 0.7   remainder
                }),
            ),
            SubMsg::new(WasmMsg::Execute {
                contract_addr: "kernel".to_string(),
                msg: to_binary(&KernelExecuteMsg::AMPReceive(pkt)).unwrap(),
                funds: vec![Coin::new(1000, "uluna"), Coin::new(2000, "uluna")],
            }),
        ])
        .add_attributes(vec![attr("action", "send"), attr("sender", "creator")]);

    assert_eq!(res, expected_res);
}

#[test]
fn test_handle_packet_exit_with_error_true() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let _res: Response = init(deps.as_mut(), None);

    let sender_funds_amount = 0u128;
    let info = mock_info(OWNER, &[Coin::new(sender_funds_amount, "uluna")]);

    let recip_address1 = "address1".to_string();
    let recip_percent1 = 10; // 10%

    let recip_address2 = "address2".to_string();
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
    let pkt = AMPPkt::new(
        info.clone().sender,
        "cosmos2contract",
        vec![
            AMPMsg::new(
                recip_address1,
                to_binary(&ExecuteMsg::Send {}).unwrap(),
                Some(vec![Coin::new(0, "uluna")]),
            ),
            AMPMsg::new(
                recip_address2,
                to_binary(&ExecuteMsg::Send {}).unwrap(),
                Some(vec![Coin::new(0, "uluna")]),
            ),
        ],
    );
    let msg = ExecuteMsg::AMPReceive(pkt);

    let splitter = Splitter {
        recipients: recipient,
        lock: Expiration::AtTime(Timestamp::from_seconds(0)),
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
fn test_execute_send_ado_recipient_exit_with_error_false() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let _res: Response = init(deps.as_mut(), None);

    let sender_funds_amount = 0u128;
    let owner = "creator";
    let info = mock_info(owner, &[Coin::new(sender_funds_amount, "uluna")]);

    let recip_address1 = "address1".to_string();
    let recip_percent1 = 10; // 10%

    let recip_address2 = "address2".to_string();
    let recip_percent2 = 20; // 20%

    let pkt = AMPPkt::new(
        info.clone().sender,
        "cosmos2contract",
        vec![
            AMPMsg::new(
                recip_address1.clone(),
                to_binary(&ExecuteMsg::Send {}).unwrap(),
                Some(vec![Coin::new(0, "uluna")]),
            ),
            AMPMsg::new(
                recip_address2.clone(),
                to_binary(&ExecuteMsg::Send {}).unwrap(),
                Some(vec![Coin::new(0, "uluna")]),
            ),
        ],
    );
    let msg = ExecuteMsg::AMPReceive(pkt);

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

    let splitter = Splitter {
        recipients: recipient,
        lock: Expiration::AtTime(Timestamp::from_seconds(0)),
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

    let pkt = AMPPkt::new(
        info.sender,
        "cosmos2contract",
        vec![AMPMsg::new(
            recip_address2,
            to_binary(&ExecuteMsg::Send {}).unwrap(),
            Some(vec![Coin::new(0, "uluna")]),
        )],
    );

    let expected_res = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "kernel".to_string(),
        msg: to_binary(&AMPExecuteMsg(pkt)).unwrap(),
        funds: coins(0, "uluna"),
    }));

    assert_eq!(res.messages[0], expected_res);
}

// #[test]
// fn test_query_splitter() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let env = mock_env();
//     let splitter = Splitter {
//         recipients: vec![],
//         lock: Expiration::AtTime(Timestamp::from_seconds(0)),
//     };

//     SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

//     let query_msg = QueryMsg::GetSplitterConfig {};
//     let res = query(deps.as_ref(), env, query_msg).unwrap();
//     let val: GetSplitterConfigResponse = from_binary(&res).unwrap();

//     assert_eq!(val.config, splitter);
// }

// #[test]
// fn test_execute_send_error() {
//     //Executes send with more than 5 tokens [ACK-04]
//     let mut deps = mock_dependencies_custom(&[]);
//     let env = mock_env();

//     let sender_funds_amount = 10000u128;
//     let owner = "creator";
//     let info = mock_info(
//         owner,
//         &vec![
//             Coin::new(sender_funds_amount, "uluna"),
//             Coin::new(sender_funds_amount, "uluna"),
//             Coin::new(sender_funds_amount, "uluna"),
//             Coin::new(sender_funds_amount, "uluna"),
//             Coin::new(sender_funds_amount, "uluna"),
//             Coin::new(sender_funds_amount, "uluna"),
//         ],
//     );

//     let recip_address1 = "address1".to_string();
//     let recip_percent1 = 10; // 10%

//     let recip_address2 = "address2".to_string();
//     let recip_percent2 = 20; // 20%

//     let recipient = vec![
//         AddressPercent {
//             recipient: Recipient::from_string(recip_address1),
//             percent: Decimal::percent(recip_percent1),
//         },
//         AddressPercent {
//             recipient: Recipient::from_string(recip_address2),
//             percent: Decimal::percent(recip_percent2),
//         },
//     ];
//     let msg = ExecuteMsg::Send {
//         reply_gas: ReplyGasExit {
//             reply_on: None,
//             gas_limit: None,
//             exit_at_error: Some(true),
//         },
//         packet: None,
//     };

//     let splitter = Splitter {
//         recipients: recipient,
//         lock: Expiration::AtTime(Timestamp::from_seconds(0)),
//     };

//     SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

//     let deps_mut = deps.as_mut();
//     ADOContract::default()
//         .instantiate(
//             deps_mut.storage,
//             mock_env(),
//             deps_mut.api,
//             mock_info(owner, &[]),
//             BaseInstantiateMsg {
//                 ado_type: "splitter".to_string(),
//                 ado_version: CONTRACT_VERSION.to_string(),
//                 operators: None,
//                 modules: None,
//                 kernel_address: None,
//             },
//         )
//         .unwrap();

//     let res = execute(deps.as_mut(), env, info, msg).unwrap_err();

//     let expected_res = ContractError::ExceedsMaxAllowedCoins {};

//     assert_eq!(res, expected_res);
// }

// #[test]
// fn test_modules() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let env = mock_env();
//     let info = mock_info("creator", &[]);
//     let msg = InstantiateMsg {
//         modules: Some(vec![Module {
//             name: Some("address_list".to_string()),
//             is_mutable: false,
//             address: MOCK_ADDRESSLIST_CONTRACT.to_owned(),
//         }]),
//         recipients: vec![AddressPercent {
//             recipient: Recipient::from_string(String::from("Some Address")),
//             percent: Decimal::percent(100),
//         }],
//         lock_time: Some(100_000),
//         kernel_address: Some("kernel_address".to_string()),
//     };
//     let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
//     let expected_res = Response::new()
//         .add_attribute("action", "register_module")
//         .add_attribute("module_idx", "1")
//         .add_attribute("method", "instantiate")
//         .add_attribute("type", "splitter");
//     assert_eq!(expected_res, res);

//     let msg = ExecuteMsg::Send {
//         reply_gas: ReplyGasExit {
//             reply_on: None,
//             gas_limit: None,
//             exit_at_error: Some(true),
//         },
//         packet: None,
//     };
//     let info = mock_info("anyone", &coins(100, "uusd"));

//     let res = execute(deps.as_mut(), mock_env(), info, msg.clone());

//     assert_eq!(
//         ContractError::Std(StdError::generic_err(
//             "Querier contract error: InvalidAddress"
//         ),),
//         res.unwrap_err()
//     );

//     let info = mock_info("sender", &coins(100, "uusd"));
//     let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     assert_eq!(
//         Response::new()
//             .add_message(BankMsg::Send {
//                 to_address: "Some Address".to_string(),
//                 amount: coins(100, "uusd"),
//             })
//             .add_attribute("action", "send")
//             .add_attribute("sender", "sender"),
//         res
//     );
// }

// #[test]
// fn test_update_app_contract() {
//     let mut deps = mock_dependencies_custom(&[]);

//     let modules: Vec<Module> = vec![Module {
//         module_name: Some("address_list".to_string()),
//         address: MOCK_ADDRESSLIST_CONTRACT.to_owned(),

//         is_mutable: false,
//     }];

//     let info = mock_info("app_contract", &[]);
//     let msg = InstantiateMsg {
//         modules: Some(modules),
//         recipients: vec![
//             AddressPercent {
//                 recipient: Recipient::from_string(String::from("Some Address")),
//                 percent: Decimal::percent(50),
//             },
//             AddressPercent {
//                 recipient: Recipient::ADO(ADORecipient {
//                     address: "eee".to_string(),
//                     msg: None,
//                 }),
//                 percent: Decimal::percent(50),
//             },
//         ],
//         lock_time: None,
//         kernel_address: None,
//     };

//     let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//     let msg = ExecuteMsg::AndrReceive(AndromedaMsg::UpdateAppContract {
//         address: "app_contract".to_string(),
//     });

//     let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     assert_eq!(
//         Response::new()
//             .add_attribute("action", "update_app_contract")
//             .add_attribute("address", "app_contract"),
//         res
//     );
// }

// #[test]
// fn test_update_app_contract_invalid_recipient() {
//     let mut deps = mock_dependencies_custom(&[]);

//     let modules: Vec<Module> = vec![Module {
//         module_name: Some("address_list".to_string()),
//         address: MOCK_ADDRESSLIST_CONTRACT.to_owned(),

//         is_mutable: false,
//     }];

//     let info = mock_info("app_contract", &[]);
//     let msg = InstantiateMsg {
//         modules: Some(modules),
//         recipients: vec![AddressPercent {
//             recipient: Recipient::ADO(ADORecipient {
//                 address: "z".to_string(),
//                 msg: None,
//             }),
//             percent: Decimal::percent(100),
//         }],
//         lock_time: None,
//         kernel_address: None,
//     };

//     let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//     let msg = ExecuteMsg::AndrReceive(AndromedaMsg::UpdateAppContract {
//         address: "app_contract".to_string(),
//     });

//     let res = execute(deps.as_mut(), mock_env(), info, msg);

//     // assert_eq!(
//     //     ContractError::InvalidComponent {
//     //         name: "z".to_string()
//     //     },
//     //     res.unwrap_err()
//     // );
//     assert!(res.is_err())
// }
