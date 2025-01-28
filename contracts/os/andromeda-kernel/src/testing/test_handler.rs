use crate::{execute::MsgHandler, state::KERNEL_ADDRESSES};
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
                vec![],
            )
            .to_sub_msg(MOCK_APP_CONTRACT, None, ReplyId::AMPMsg.repr())
            .unwrap(),
            expected_error: None,
        },
        TestHandleLocalCase {
            name: "Valid message to ADO (no funds)",
            sender: "sender",
            msg: AMPMsg::new(MOCK_APP_CONTRACT, to_json_binary(&true).unwrap(), None),
            ctx: Some(AMPCtx::new("origin", MOCK_APP_CONTRACT, 1, None)),
            expected_submessage: AMPPkt::new(
                "origin",
                "sender",
                vec![AMPMsg::new(
                    MOCK_APP_CONTRACT,
                    to_json_binary(&true).unwrap(),
                    None,
                )],
                vec![],
            )
            .to_sub_msg(MOCK_APP_CONTRACT, None, ReplyId::AMPMsg.repr())
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
            ctx: Some(AMPCtx::new("origin", MOCK_APP_CONTRACT, 1, None)),
            expected_submessage: AMPPkt::new(
                "origin",
                "sender",
                vec![AMPMsg::new(
                    MOCK_APP_CONTRACT,
                    to_json_binary(&true).unwrap(),
                    Some(vec![coin(100, "denom"), coin(200, "denom_two")]),
                )],
                vec![],
            )
            .to_sub_msg(
                MOCK_APP_CONTRACT,
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
                error: Some("No message or funds supplied".to_string()),
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
                vec![],
            )
            .to_sub_msg(MOCK_APP_CONTRACT, None, ReplyId::AMPMsg.repr())
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
            MsgHandler::new(test.msg).handle_local(deps.as_mut(), info, mock_env(), test.ctx, 0);

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
