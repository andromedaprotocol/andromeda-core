use crate::execute::generate_or_validate_packet_id;
#[cfg(test)]
use crate::{execute::handle_local, state::KERNEL_ADDRESSES};
use andromeda_std::{
    amp::{
        messages::{AMPCtx, AMPMsg, AMPMsgConfig, AMPPkt},
        ADO_DB_KEY,
    },
    common::reply::ReplyId,
    error::ContractError,
    testing::mock_querier::{
        mock_dependencies_custom, FAKE_VFS_PATH, INVALID_CONTRACT, MOCK_ADODB_CONTRACT,
        MOCK_APP_CONTRACT, MOCK_WALLET,
    },
};
use cosmwasm_std::{
    coin,
    testing::{mock_env, mock_info},
    to_json_binary, Addr, BankMsg, Binary, ReplyOn, SubMsg,
};

struct TestHandleLocalCase {
    name: &'static str,
    sender: &'static str,
    msg: AMPMsg,
    ctx: Option<AMPCtx>,
    expected_submessage: SubMsg,
    expected_error: Option<ContractError>,
}

#[test]
fn test_handle_local() {
    fn create_test_msg_with_config(config: AMPMsgConfig) -> AMPMsg {
        let base_msg = AMPMsg::new(MOCK_APP_CONTRACT, to_json_binary(&true).unwrap(), None);
        base_msg.with_config(config)
    }

    // Then in tests:
    let config = AMPMsgConfig {
        reply_on: ReplyOn::Error,
        exit_at_error: false,
        gas_limit: Some(1000000),
        direct: false,
        ibc_config: None,
    };

    let test_cases = vec![
        TestHandleLocalCase {
            name: "Valid message to ADO (no funds/context)",
            sender: "sender",
            msg: AMPMsg::new(MOCK_APP_CONTRACT, to_json_binary(&true).unwrap(), None),
            ctx: None,
            expected_submessage: AMPPkt::new(
                "sender",
                "sender",
                vec![AMPMsg::new(
                    MOCK_APP_CONTRACT,
                    to_json_binary(&true).unwrap(),
                    None,
                )],
            )
            .to_sub_msg(
                Addr::unchecked(MOCK_APP_CONTRACT),
                None,
                ReplyId::AMPMsg.repr(),
            )
            .unwrap(),
            expected_error: None,
        },
        TestHandleLocalCase {
            name: "Valid message to ADO (no funds)",
            sender: "sender",
            msg: AMPMsg::new(MOCK_APP_CONTRACT, to_json_binary(&true).unwrap(), None),
            ctx: Some(AMPCtx::new("origin", MOCK_APP_CONTRACT, None)),
            expected_submessage: AMPPkt::new(
                "origin",
                "sender",
                vec![AMPMsg::new(
                    MOCK_APP_CONTRACT,
                    to_json_binary(&true).unwrap(),
                    None,
                )],
            )
            .to_sub_msg(
                Addr::unchecked(MOCK_APP_CONTRACT),
                None,
                ReplyId::AMPMsg.repr(),
            )
            .unwrap(),
            expected_error: None,
        },
        TestHandleLocalCase {
            name: "Valid message to ADO w/ funds",
            sender: "sender",
            msg: AMPMsg::new(
                MOCK_APP_CONTRACT,
                to_json_binary(&true).unwrap(),
                Some(vec![coin(100, "denom"), coin(200, "denom_two")]),
            ),
            ctx: Some(AMPCtx::new("origin", MOCK_APP_CONTRACT, None)),
            expected_submessage: AMPPkt::new(
                "origin",
                "sender",
                vec![AMPMsg::new(
                    MOCK_APP_CONTRACT,
                    to_json_binary(&true).unwrap(),
                    Some(vec![coin(100, "denom"), coin(200, "denom_two")]),
                )],
            )
            .to_sub_msg(
                Addr::unchecked(MOCK_APP_CONTRACT),
                Some(vec![coin(100, "denom"), coin(200, "denom_two")]),
                ReplyId::AMPMsg.repr(),
            )
            .unwrap(),
            expected_error: None,
        },
        TestHandleLocalCase {
            name: "Valid message direct to Non-ADO (no funds)",
            sender: "sender",
            msg: AMPMsg::new(INVALID_CONTRACT, to_json_binary(&true).unwrap(), None),
            ctx: None,
            expected_submessage: AMPMsg::new(
                INVALID_CONTRACT,
                to_json_binary(&true).unwrap(),
                None,
            )
            .generate_sub_msg_direct(Addr::unchecked(INVALID_CONTRACT), ReplyId::AMPMsg.repr()),
            expected_error: None,
        },
        TestHandleLocalCase {
            name: "Valid message direct to Non-ADO w/ funds",
            sender: "sender",
            msg: AMPMsg::new(
                INVALID_CONTRACT,
                to_json_binary(&true).unwrap(),
                Some(vec![coin(100, "denom"), coin(200, "denom_two")]),
            ),
            ctx: None,
            expected_submessage: AMPMsg::new(
                INVALID_CONTRACT,
                to_json_binary(&true).unwrap(),
                Some(vec![coin(100, "denom"), coin(200, "denom_two")]),
            )
            .generate_sub_msg_direct(Addr::unchecked(INVALID_CONTRACT), ReplyId::AMPMsg.repr()),
            expected_error: None,
        },
        TestHandleLocalCase {
            name: "Recipient not a contract",
            sender: "sender",
            msg: AMPMsg::new(
                MOCK_WALLET,
                to_json_binary(&true).unwrap(),
                Some(vec![coin(100, "denom"), coin(200, "denom_two")]),
            ),
            ctx: None,
            expected_submessage: AMPMsg::new(
                INVALID_CONTRACT,
                to_json_binary(&true).unwrap(),
                Some(vec![coin(100, "denom"), coin(200, "denom_two")]),
            )
            .generate_sub_msg_direct(Addr::unchecked(INVALID_CONTRACT), ReplyId::AMPMsg.repr()),
            expected_error: Some(ContractError::InvalidPacket {
                error: Some("Recipient is not a contract".to_string()),
            }),
        },
        TestHandleLocalCase {
            name: "Invalid Recipient Path",
            sender: "sender",
            msg: AMPMsg::new(
                FAKE_VFS_PATH,
                to_json_binary(&true).unwrap(),
                Some(vec![coin(100, "denom"), coin(200, "denom_two")]),
            ),
            ctx: None,
            expected_submessage: AMPMsg::new(
                FAKE_VFS_PATH,
                to_json_binary(&true).unwrap(),
                Some(vec![coin(100, "denom"), coin(200, "denom_two")]),
            )
            .generate_sub_msg_direct(Addr::unchecked(INVALID_CONTRACT), ReplyId::AMPMsg.repr()),
            expected_error: Some(ContractError::InvalidPathname {
                error: Some(format!(
                    "{:?} does not exist in the file system",
                    FAKE_VFS_PATH
                )),
            }),
        },
        TestHandleLocalCase {
            name: "Valid bank send message",
            sender: "sender",
            msg: AMPMsg::new(
                "receiver",
                Binary::default(),
                Some(vec![coin(100, "denom"), coin(200, "denom_two")]),
            ),
            ctx: None,
            expected_submessage: SubMsg::reply_on_error(
                BankMsg::Send {
                    to_address: "receiver".to_string(),
                    amount: vec![coin(100, "denom"), coin(200, "denom_two")],
                },
                ReplyId::AMPMsg.repr(),
            ),
            expected_error: None,
        },
        TestHandleLocalCase {
            name: "Bank send no funds",
            sender: "sender",
            msg: AMPMsg::new("receiver", Binary::default(), None),
            ctx: None,
            expected_submessage: SubMsg::reply_on_error(
                BankMsg::Send {
                    to_address: "receiver".to_string(),
                    amount: vec![],
                },
                ReplyId::AMPMsg.repr(),
            ),
            expected_error: Some(ContractError::InvalidPacket {
                error: Some("No funds supplied".to_string()),
            }),
        },
        TestHandleLocalCase {
            name: "Message with custom reply configuration",
            sender: "sender",
            msg: create_test_msg_with_config(config.clone()),
            ctx: None,
            expected_submessage: AMPPkt::new(
                "sender",
                "sender",
                vec![create_test_msg_with_config(config)],
            )
            .to_sub_msg(
                Addr::unchecked(MOCK_APP_CONTRACT),
                None,
                ReplyId::AMPMsg.repr(),
            )
            .unwrap(),
            expected_error: None,
        },
    ];

    for test in test_cases {
        let mut deps = mock_dependencies_custom(&[]);
        let info = mock_info(test.sender, &[]);

        KERNEL_ADDRESSES
            .save(
                deps.as_mut().storage,
                ADO_DB_KEY,
                &Addr::unchecked(MOCK_ADODB_CONTRACT),
            )
            .unwrap();

        let res: Result<cosmwasm_std::Response, ContractError> =
            handle_local(deps.as_mut(), info, mock_env(), test.ctx, test.msg);

        if let Some(err) = test.expected_error {
            assert_eq!(res.unwrap_err(), err, "{}", test.name);
            continue;
        }

        let response = res.unwrap();

        assert_eq!(
            response.messages[0], test.expected_submessage,
            "{}",
            test.name
        );
    }
}

#[test]
fn test_generate_or_validate_packet_id() {
    use crate::state::TX_INDEX;
    use cosmwasm_std::{testing::mock_env, Uint128};

    // Initialize dependencies
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    // Test case 1: Generate new packet ID when none exists
    // Initialize TX_INDEX to 0
    TX_INDEX
        .save(deps.as_mut().storage, &Uint128::zero())
        .unwrap();

    let result = generate_or_validate_packet_id(&mut deps.as_mut(), &env, None).unwrap();

    // Verify the generated packet ID format
    let parts: Vec<&str> = result.split('.').collect();
    assert_eq!(parts[2], "0"); // Should be 0 since it's the first message

    // Verify TX_INDEX was incremented
    let new_tx_index = TX_INDEX.load(deps.as_ref().storage).unwrap();
    assert_eq!(new_tx_index, Uint128::one());

    // Test case 2: Generate another packet ID
    let result2 = generate_or_validate_packet_id(&mut deps.as_mut(), &env, None).unwrap();

    // Verify the generated packet ID format
    let parts: Vec<&str> = result2.split('.').collect();
    assert_eq!(parts[2], "1"); // Should be 1 since it's the second message

    // Verify TX_INDEX was incremented again
    let new_tx_index = TX_INDEX.load(deps.as_ref().storage).unwrap();
    assert_eq!(new_tx_index, Uint128::new(2));

    // Test case 3: Validate existing packet ID from same chain
    let existing_id = format!(
        "{}.{}.{}",
        env.block.chain_id,
        env.block.height,
        Uint128::new(1)
    );
    let result3 =
        generate_or_validate_packet_id(&mut deps.as_mut(), &env, Some(existing_id.clone()))
            .unwrap();

    // Verify the validated packet ID matches the input
    assert_eq!(result3, existing_id);

    // Test case 4: Validate existing packet ID from different chain
    let different_chain_id = "different-chain";
    let existing_id_diff_chain = format!(
        "{}.{}.{}",
        different_chain_id,
        env.block.height,
        // The id isn't checked from other chains
        Uint128::new(10)
    );
    let result4 = generate_or_validate_packet_id(
        &mut deps.as_mut(),
        &env,
        Some(existing_id_diff_chain.clone()),
    )
    .unwrap();

    // Verify the validated packet ID matches the input
    assert_eq!(result4, existing_id_diff_chain);
}
