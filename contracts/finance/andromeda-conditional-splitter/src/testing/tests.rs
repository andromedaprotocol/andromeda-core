use andromeda_std::{
    amp::{
        messages::{AMPMsg, AMPPkt},
        recipient::Recipient,
    },
    common::Milliseconds,
    error::ContractError,
};
use andromeda_testing::economics_msg::generate_economics_message;
use cosmwasm_std::{
    attr, from_json,
    testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR},
    to_json_binary, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Response, SubMsg, Uint128,
};
pub const OWNER: &str = "creator";

use super::mock_querier::MOCK_KERNEL_CONTRACT;

use crate::{
    contract::{execute, instantiate, query},
    state::{CONDITIONAL_SPLITTER, FUNDS_DISTRIBUTED},
    testing::mock_querier::mock_dependencies_custom,
};
use andromeda_finance::{
    conditional_splitter::{
        ConditionalSplitter, ExecuteMsg, GetConditionalSplitterConfigResponse, InstantiateMsg,
        QueryMsg, Threshold,
    },
    splitter::AddressPercent,
};

fn init(deps: DepsMut) -> Response {
    let msg = InstantiateMsg {
        owner: Some(OWNER.to_owned()),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        thresholds: vec![
            Threshold::new(
                Uint128::zero(),
                vec![AddressPercent::new(
                    Recipient::from_string(String::from("some_address")),
                    Decimal::from_ratio(Uint128::one(), Uint128::new(2)),
                )],
            ),
            Threshold::new(
                Uint128::new(11),
                vec![AddressPercent::new(
                    Recipient::from_string(String::from("some_address")),
                    Decimal::from_ratio(Uint128::one(), Uint128::new(2)),
                )],
            ),
        ],
        lock_time: Some(Milliseconds::from_seconds(100_000)),
    };

    let info = mock_info("owner", &[]);
    instantiate(deps, mock_env(), info, msg).unwrap()
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);
    let res = init(deps.as_mut());
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_execute_update_lock() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut());

    let env = mock_env();

    let current_time = env.block.time.seconds();
    let lock_time = 100_000;

    // Start off with an expiration that's behind current time (expired)
    let splitter = ConditionalSplitter {
        lock: Milliseconds::from_seconds(current_time - 1),
        thresholds: vec![Threshold {
            min: Uint128::zero(),
            address_percent: vec![],
        }],
    };

    CONDITIONAL_SPLITTER
        .save(deps.as_mut().storage, &splitter)
        .unwrap();

    let msg = ExecuteMsg::UpdateLock {
        lock_time: Milliseconds::from_seconds(lock_time),
    };

    let info = mock_info(OWNER, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    let new_lock = Milliseconds::from_seconds(current_time + lock_time);
    assert_eq!(
        Response::default()
            .add_attributes(vec![
                attr("action", "update_lock"),
                attr("locked", new_lock.to_string())
            ])
            .add_submessage(generate_economics_message(OWNER, "UpdateLock")),
        res
    );

    //check result
    let splitter = CONDITIONAL_SPLITTER.load(deps.as_ref().storage).unwrap();
    assert!(!splitter.lock.is_expired(&env.block));
    assert_eq!(new_lock, splitter.lock);
}

// #[test]
// fn test_execute_update_recipients() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let env = mock_env();
//     let _res = init(deps.as_mut());

//     let splitter = ConditionalSplitter {
//         recipients: vec![],
//         lock: Milliseconds::from_seconds(0),
//         thresholds: vec![Threshold::new(Uint128::zero())],
//     };

//     CONDITIONAL_SPLITTER
//         .save(deps.as_mut().storage, &splitter)
//         .unwrap();

//     // Duplicate recipients
//     let duplicate_recipients = vec![
//         AddressPercentages {
//             recipient: Recipient::from_string(String::from("addr1")),
//             percentages: vec![Decimal::percent(40)],
//         },
//         AddressPercentages {
//             recipient: Recipient::from_string(String::from("addr1")),
//             percentages: vec![Decimal::percent(60)],
//         },
//     ];
//     let msg = ExecuteMsg::UpdateRecipients {
//         recipients: duplicate_recipients,
//     };

//     let info = mock_info(OWNER, &[]);
//     let res = execute(deps.as_mut(), env.clone(), info, msg);
//     assert_eq!(ContractError::DuplicateRecipient {}, res.unwrap_err());

//     let recipients = vec![
//         AddressFunds {
//             recipient: Recipient::from_string(String::from("addr1")),
//             percent: Decimal::percent(40),
//         },
//         AddressFunds {
//             recipient: Recipient::from_string(String::from("addr2")),
//             percent: Decimal::percent(60),
//         },
//     ];
//     let msg = ExecuteMsg::UpdateRecipients {
//         recipients: recipients.clone(),
//     };

//     let info = mock_info("incorrect_owner", &[]);
//     let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
//     assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

//     let info = mock_info(OWNER, &[]);
//     let res = execute(deps.as_mut(), env, info, msg).unwrap();
//     assert_eq!(
//         Response::default()
//             .add_attributes(vec![attr("action", "update_recipients")])
//             .add_submessage(generate_economics_message(OWNER, "UpdateRecipients")),
//         res
//     );

//     //check result
//     let splitter = CONDITIONAL_SPLITTER.load(deps.as_ref().storage).unwrap();
//     assert_eq!(splitter.recipients, recipients);
// }

#[test]
fn test_execute_send() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let recip_address1 = "address1".to_string();
    let recip_address2 = "address2".to_string();

    let second_threshold = Uint128::new(10);

    let recip1 = Recipient::from_string(recip_address1);
    let recip2 = Recipient::from_string(recip_address2);

    let msg = InstantiateMsg {
        owner: Some(OWNER.to_owned()),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        thresholds: vec![
            Threshold::new(
                Uint128::zero(),
                vec![
                    AddressPercent::new(
                        recip1.clone(), // 50%
                        Decimal::from_ratio(Uint128::one(), Uint128::new(2)),
                    ),
                    AddressPercent::new(
                        recip2.clone(), // 20%
                        Decimal::from_ratio(Uint128::one(), Uint128::new(5)),
                    ),
                ],
            ),
            Threshold::new(
                second_threshold,
                vec![
                    AddressPercent::new(
                        recip1.clone(), // 20%
                        Decimal::from_ratio(Uint128::one(), Uint128::new(5)),
                    ),
                    AddressPercent::new(
                        recip2.clone(), // 10%
                        Decimal::from_ratio(Uint128::one(), Uint128::new(10)),
                    ),
                ],
            ),
            Threshold::new(
                Uint128::new(50),
                vec![
                    AddressPercent::new(
                        recip1.clone(), // 50%
                        Decimal::from_ratio(Uint128::one(), Uint128::new(2)),
                    ),
                    AddressPercent::new(
                        recip2.clone(), // 50%
                        Decimal::from_ratio(Uint128::one(), Uint128::new(2)),
                    ),
                ],
            ),
        ],
        lock_time: Some(Milliseconds::from_seconds(100_000)),
    };

    let info = mock_info("owner", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // First batch of funds which will put funds received to 10 and push the next batch to the other threshold
    let first_batch = 10u128;

    // Second batch will be used to test the second threshold
    let second_batch = 10u128;

    // Third batch will be used to test crossing a threshold
    let third_batch = 100u128;

    // First batch
    let info = mock_info(OWNER, &[Coin::new(first_batch, "uandr")]);
    let msg = ExecuteMsg::Send {};
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // 50 percent
    let amp_msg_1 = recip1
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(5, "uandr")]))
        .unwrap();
    // 20 percent
    let amp_msg_2 = recip2
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(2, "uandr")]))
        .unwrap();
    let amp_pkt = AMPPkt::new(
        MOCK_CONTRACT_ADDR.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        vec![amp_msg_1, amp_msg_2],
    );
    let amp_msg = amp_pkt
        .to_sub_msg(
            MOCK_KERNEL_CONTRACT,
            Some(vec![Coin::new(5, "uandr"), Coin::new(2, "uandr")]),
            1,
        )
        .unwrap();

    let expected_res = Response::new()
        .add_submessages(vec![
            SubMsg::new(
                // refunds remainder to sender
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: OWNER.to_string(),
                    amount: vec![Coin::new(3, "uandr")], // 10 - (0.5 * 10) - (0.2 * 10)   remainder
                }),
            ),
            amp_msg,
        ])
        .add_attributes(vec![attr("action", "send"), attr("sender", "creator")])
        .add_submessage(generate_economics_message(OWNER, "Send"));

    assert_eq!(res, expected_res);

    let funds_distributed = FUNDS_DISTRIBUTED.load(&deps.storage).unwrap();
    let expected_funds_distributed = Uint128::new(10);
    assert_eq!(funds_distributed, expected_funds_distributed);

    // Second batch
    let info = mock_info(OWNER, &[Coin::new(second_batch, "uandr")]);
    let msg = ExecuteMsg::Send {};
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let funds_distributed = FUNDS_DISTRIBUTED.load(&deps.storage).unwrap();
    let expected_funds_distributed = Uint128::new(10 + 10);
    assert_eq!(funds_distributed, expected_funds_distributed);

    // 20 percent
    let amp_msg_1 = recip1
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(2, "uandr")]))
        .unwrap();
    // 10 percent
    let amp_msg_2 = recip2
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(1, "uandr")]))
        .unwrap();
    let amp_pkt = AMPPkt::new(
        MOCK_CONTRACT_ADDR.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        vec![amp_msg_1, amp_msg_2],
    );
    let amp_msg = amp_pkt
        .to_sub_msg(
            MOCK_KERNEL_CONTRACT,
            Some(vec![Coin::new(2, "uandr"), Coin::new(1, "uandr")]),
            1,
        )
        .unwrap();

    let expected_res = Response::new()
        .add_submessages(vec![
            SubMsg::new(
                // refunds remainder to sender
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: OWNER.to_string(),
                    amount: vec![Coin::new(7, "uandr")], // 10 - (0.2 * 10) - (0.1 * 10)   remainder
                }),
            ),
            amp_msg,
        ])
        .add_attributes(vec![attr("action", "send"), attr("sender", "creator")])
        .add_submessage(generate_economics_message(OWNER, "Send"));

    assert_eq!(res, expected_res);

    // Third batch
    let info = mock_info(OWNER, &[Coin::new(third_batch, "uandr")]);
    let msg = ExecuteMsg::Send {};
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let funds_distributed = FUNDS_DISTRIBUTED.load(&deps.storage).unwrap();
    let expected_funds_distributed = Uint128::new(10 + 10 + 100);
    assert_eq!(funds_distributed, expected_funds_distributed);

    // amount of 30 * 20% + 70 * 50% = 41
    let amp_msg_1 = recip1
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(41, "uandr")]))
        .unwrap();
    // amount of 30 * 10% + 70 * 50% = 38
    let amp_msg_2 = recip2
        .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(38, "uandr")]))
        .unwrap();
    let amp_pkt = AMPPkt::new(
        MOCK_CONTRACT_ADDR.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        vec![amp_msg_1, amp_msg_2],
    );
    let amp_msg = amp_pkt
        .to_sub_msg(
            MOCK_KERNEL_CONTRACT,
            Some(vec![Coin::new(41, "uandr"), Coin::new(38, "uandr")]),
            1,
        )
        .unwrap();

    let expected_res = Response::new()
        .add_submessages(vec![
            SubMsg::new(
                // refunds remainder to sender
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: OWNER.to_string(),
                    amount: vec![Coin::new(21, "uandr")], // 100 - 41 - 38  remainder
                }),
            ),
            amp_msg,
        ])
        .add_attributes(vec![attr("action", "send"), attr("sender", "creator")])
        .add_submessage(generate_economics_message(OWNER, "Send"));

    assert_eq!(res, expected_res);
}

// #[test]
// fn test_execute_send_ado_recipient() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let env = mock_env();
//     let _res: Response = init(deps.as_mut());

//     let sender_funds_amount = 10000u128;
//     let info = mock_info(OWNER, &[Coin::new(sender_funds_amount, "uluna")]);

//     let recip_address1 = "address1".to_string();
//     let recip_percent1 = 10; // 10%

//     let recip_address2 = "address2".to_string();
//     let recip_percent2 = 20; // 20%

//     let recip1 = Recipient::from_string(recip_address1);
//     let recip2 = Recipient::from_string(recip_address2);

//     let recipient = vec![
//         AddressFunds {
//             recipient: recip1.clone(),
//             percent: Decimal::percent(recip_percent1),
//         },
//         AddressFunds {
//             recipient: recip2.clone(),
//             percent: Decimal::percent(recip_percent2),
//         },
//     ];
//     let msg = ExecuteMsg::Send {};

//     let amp_msg_1 = recip1
//         .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(1000, "uluna")]))
//         .unwrap();
//     let amp_msg_2 = recip2
//         .generate_amp_msg(&deps.as_ref(), Some(vec![Coin::new(2000, "uluna")]))
//         .unwrap();
//     let amp_pkt = AMPPkt::new(
//         MOCK_CONTRACT_ADDR.to_string(),
//         MOCK_CONTRACT_ADDR.to_string(),
//         vec![amp_msg_1, amp_msg_2],
//     );
//     let amp_msg = amp_pkt
//         .to_sub_msg(
//             MOCK_KERNEL_CONTRACT,
//             Some(vec![Coin::new(1000, "uluna"), Coin::new(2000, "uluna")]),
//             1,
//         )
//         .unwrap();

//     let splitter = ConditionalSplitter {
//         recipients: recipient,
//         lock: Milliseconds::default(),
//     };

//     CONDITIONAL_SPLITTER
//         .save(deps.as_mut().storage, &splitter)
//         .unwrap();

//     let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

//     let expected_res = Response::new()
//         .add_submessages(vec![
//             SubMsg::new(
//                 // refunds remainder to sender
//                 CosmosMsg::Bank(BankMsg::Send {
//                     to_address: info.sender.to_string(),
//                     amount: vec![Coin::new(7000, "uluna")], // 10000 * 0.7   remainder
//                 }),
//             ),
//             amp_msg,
//         ])
//         .add_attribute("action", "send")
//         .add_attribute("sender", "creator")
//         .add_submessage(generate_economics_message(OWNER, "Send"));

//     assert_eq!(res, expected_res);
// }

// #[test]
// fn test_handle_packet_exit_with_error_true() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let env = mock_env();
//     let _res: Response = init(deps.as_mut());

//     let sender_funds_amount = 0u128;
//     let info = mock_info(OWNER, &[Coin::new(sender_funds_amount, "uluna")]);

//     let recip_address1 = "address1".to_string();
//     let recip_percent1 = 10; // 10%

//     let recip_percent2 = 20; // 20%

//     let recipient = vec![
//         AddressPercentages {
//             recipient: Recipient::from_string(recip_address1.clone()),
//             percentages: vec![Decimal::percent(recip_percent1)],
//         },
//         AddressPercentages {
//             recipient: Recipient::from_string(recip_address1.clone()),
//             percentages: vec![Decimal::percent(recip_percent2)],
//         },
//     ];
//     let pkt = AMPPkt::new(
//         info.clone().sender,
//         "cosmos2contract",
//         vec![AMPMsg::new(
//             recip_address1,
//             to_json_binary(&ExecuteMsg::Send {}).unwrap(),
//             Some(vec![Coin::new(0, "uluna")]),
//         )],
//     );
//     let msg = ExecuteMsg::AMPReceive(pkt);

//     let splitter = ConditionalSplitter {
//         recipients: recipient,
//         lock: Milliseconds::default(),
//         thresholds: vec![Threshold::new(Uint128::zero())],
//     };

//     CONDITIONAL_SPLITTER
//         .save(deps.as_mut().storage, &splitter)
//         .unwrap();

//     let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

//     assert_eq!(
//         err,
//         ContractError::InvalidFunds {
//             msg: "Amount must be non-zero".to_string(),
//         }
//     );
// }

#[test]
fn test_query_splitter() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let splitter = ConditionalSplitter {
        lock: Milliseconds::default(),
        thresholds: vec![Threshold::new(Uint128::zero(), vec![])],
    };

    CONDITIONAL_SPLITTER
        .save(deps.as_mut().storage, &splitter)
        .unwrap();

    let query_msg = QueryMsg::GetConditionalSplitterConfig {};
    let res = query(deps.as_ref(), env, query_msg).unwrap();
    let val: GetConditionalSplitterConfigResponse = from_json(res).unwrap();

    assert_eq!(val.config, splitter);
}

// #[test]
// fn test_execute_send_error() {
//     //Executes send with more than 5 tokens [ACK-04]
//     let mut deps = mock_dependencies_custom(&[]);
//     let env = mock_env();
//     let _res: Response = init(deps.as_mut());

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
//         AddressFunds {
//             recipient: Recipient::from_string(recip_address1),
//             percent: Decimal::percent(recip_percent1),
//         },
//         AddressFunds {
//             recipient: Recipient::from_string(recip_address2),
//             percent: Decimal::percent(recip_percent2),
//         },
//     ];
//     let msg = ExecuteMsg::Send {};

//     let splitter = ConditionalSplitter {
//         recipients: recipient,
//         lock: Milliseconds::default(),
//     };

//     CONDITIONAL_SPLITTER
//         .save(deps.as_mut().storage, &splitter)
//         .unwrap();

//     let res = execute(deps.as_mut(), env, info, msg).unwrap_err();

//     let expected_res = ContractError::ExceedsMaxAllowedCoins {};

//     assert_eq!(res, expected_res);
// }

#[test]
fn test_update_app_contract() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res: Response = init(deps.as_mut());

    let info = mock_info(OWNER, &[]);

    let msg = ExecuteMsg::UpdateAppContract {
        address: "app_contract".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "update_app_contract")
            .add_attribute("address", "app_contract")
            .add_submessage(generate_economics_message(OWNER, "UpdateAppContract")),
        res
    );
}

#[test]
fn test_update_app_contract_invalid_recipient() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res: Response = init(deps.as_mut());

    let info = mock_info(OWNER, &[]);

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
