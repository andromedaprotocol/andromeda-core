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
        MOCK_APP_CONTRACT, MOCK_WALLET, RECEIVER,
    },
};
use cosmwasm_std::{
    coin,
    testing::{message_info, mock_env},
    to_json_binary, Addr, BankMsg, Binary, ReplyOn, SubMsg,
};
pub const SENDER: &str = "cosmwasm1pgm8hyk0pvphmlvfjc8wsvk4daluz5tgrw6pu5mfpemk74uxnx9qlm3aqg";

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
                SENDER,
                SENDER,
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
                SENDER,
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
                SENDER,
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
                RECEIVER,
                Binary::default(),
                Some(vec![coin(100, "denom"), coin(200, "denom_two")]),
            ),
            ctx: None,
            expected_submessage: SubMsg::reply_on_error(
                BankMsg::Send {
                    to_address: RECEIVER.to_string(),
                    amount: vec![coin(100, "denom"), coin(200, "denom_two")],
                },
                ReplyId::AMPMsg.repr(),
            ),
            expected_error: None,
        },
        TestHandleLocalCase {
            name: "Bank send no funds",
            sender: "sender",
            msg: AMPMsg::new(RECEIVER, Binary::default(), None),
            ctx: None,
            expected_submessage: SubMsg::reply_on_error(
                BankMsg::Send {
                    to_address: RECEIVER.to_string(),
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
                SENDER,
                SENDER,
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
        let sender = deps.api.addr_make(test.sender);
        let info = message_info(&sender, &[]);

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

use crate::state::TX_INDEX;
use cosmwasm_std::Uint128;
use rstest::*;

// Helper to reset TX_INDEX before each case
fn setup_tx_index(storage: &mut dyn cosmwasm_std::Storage, value: Uint128) {
    TX_INDEX.save(storage, &value).unwrap();
}

#[rstest]
#[case::generate_first(None, Uint128::zero(), "0", Uint128::one())]
#[case::generate_second(None, Uint128::one(), "1", Uint128::new(2))]
#[case::validate_same_chain(Some("test-chain.12345.1"), Uint128::zero(), "1", Uint128::zero())]
#[case::validate_different_chain(
    Some("different-chain.12345.10"),
    Uint128::zero(),
    "10",
    Uint128::zero()
)]
fn test_generate_or_validate_packet_id_cases(
    #[case] input: Option<&str>,
    #[case] initial_index: Uint128,
    #[case] expected_index_str: &str,
    #[case] expected_final_index: Uint128,
) {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    setup_tx_index(deps.as_mut().storage, initial_index);

    let input_string = input.map(|s| s.to_string());
    let result = generate_or_validate_packet_id(&mut deps.as_mut(), &env, input_string).unwrap();

    let parts: Vec<&str> = result.split('.').collect();
    assert_eq!(parts[2], expected_index_str);

    let final_tx_index = TX_INDEX.load(deps.as_ref().storage).unwrap();
    assert_eq!(final_tx_index, expected_final_index);
}
