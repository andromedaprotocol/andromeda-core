#[cfg(test)]
use crate::execute::handle_ibc_direct;
use crate::state::{CURR_CHAIN, KERNEL_ADDRESSES};
use andromeda_std::amp::messages::{AMPMsg, AMPMsgConfig, AMPPkt};
use andromeda_std::amp::AndrAddr;
use andromeda_std::amp::VFS_KEY;
use andromeda_std::error::ContractError;
use andromeda_std::os::kernel::{ChannelInfo, IbcExecuteMsg};
use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
use cosmwasm_std::{from_json, Addr, Binary, Coin, CosmosMsg, IbcMsg, ReplyOn, Uint128};

// Helper function to set up common test state
fn setup_test_state(
    deps: &mut cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        cosmwasm_std::testing::MockQuerier,
    >,
) {
    // Set required storage values
    CURR_CHAIN
        .save(deps.as_mut().storage, &"source-chain".to_string())
        .unwrap();

    // Set up VFS address in kernel addresses
    KERNEL_ADDRESSES
        .save(deps.as_mut().storage, VFS_KEY, &Addr::unchecked("mock_vfs"))
        .unwrap();
}

#[test]
fn test_handle_ibc_direct_success() {
    // Setup
    let mut deps = mock_dependencies();
    let env: cosmwasm_std::Env = mock_env();
    let sender = deps.api.addr_make("sender");
    let info = message_info(&sender, &[]);

    // Set up test state with mocked storage values
    setup_test_state(&mut deps);

    // Create message without funds using AMPMsg::new
    let message = AMPMsg::new(
        AndrAddr::from_string("ibc://juno-1/recipient".to_string()),
        Binary::from(b"{\"execute_something\":{}}"),
        None,
    )
    .with_config(AMPMsgConfig {
        direct: true,
        reply_on: ReplyOn::Always,
        exit_at_error: false,
        gas_limit: None,
        ibc_config: None,
    });

    // Create channel info
    let channel_info = ChannelInfo {
        direct_channel_id: Some("channel-direct".to_string()),
        ics20_channel_id: Some("channel-0".to_string()),
        kernel_address: "juno_kernel".to_string(),
        supported_modules: vec![],
    };

    // Execute handler
    let response = handle_ibc_direct(
        deps.as_mut(),
        info,
        env.clone(),
        None,
        message.clone(),
        channel_info.clone(),
    )
    .unwrap();

    // Check IBC message
    assert_eq!(response.messages.len(), 1);
    match &response.messages[0].msg {
        CosmosMsg::Ibc(IbcMsg::SendPacket {
            channel_id, data, ..
        }) => {
            assert_eq!(channel_id, "channel-direct");

            // Verify the IBC message format
            let ibc_msg: IbcExecuteMsg = from_json(data).unwrap();
            match ibc_msg {
                IbcExecuteMsg::SendMessage { amp_packet } => {
                    assert_eq!(amp_packet.messages.len(), 1);
                    // The recipient should have the protocol and chain stripped
                    assert_eq!(amp_packet.messages[0].recipient.get_raw_path(), "recipient");
                    assert_eq!(
                        amp_packet.messages[0].message,
                        Binary::from(b"{\"execute_something\":{}}")
                    );

                    // Check the context
                    assert_eq!(amp_packet.ctx.get_origin(), sender.to_string());

                    // Should have one hop with correct source and destination chains
                    assert_eq!(amp_packet.ctx.previous_hops.len(), 1);
                    assert_eq!(amp_packet.ctx.previous_hops[0].from_chain, "source-chain");
                    assert_eq!(amp_packet.ctx.previous_hops[0].to_chain, "juno-1");
                    assert_eq!(amp_packet.ctx.previous_hops[0].channel, "channel-direct");
                }
                IbcExecuteMsg::SendMessageWithFunds { .. } => {
                    panic!("Unexpected SendMessageWithFunds")
                }
                IbcExecuteMsg::CreateADO { .. } => panic!("Unexpected CreateADO"),
                IbcExecuteMsg::RegisterUsername { .. } => panic!("Unexpected RegisterUsername"),
            }
        }
        _ => panic!("Expected IBC SendPacket message"),
    }

    // Check attributes
    assert!(response
        .attributes
        .iter()
        .any(|attr| attr.key == "method" && attr.value == "execute_send_message"));
    assert!(response
        .attributes
        .iter()
        .any(|attr| attr.key == "channel" && attr.value == "channel-direct"));
    assert!(response
        .attributes
        .iter()
        .any(|attr| attr.key == "receiving_kernel_address" && attr.value == "juno_kernel"));
    assert!(response
        .attributes
        .iter()
        .any(|attr| attr.key == "chain" && attr.value == "juno-1"));
}

#[test]
fn test_handle_ibc_direct_empty_message() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let sender = deps.api.addr_make("sender");
    let info = message_info(&sender, &[]);

    // Set up test state
    setup_test_state(&mut deps);
    let message = AMPMsg::new(
        AndrAddr::from_string("ibc://juno-1/recipient".to_string()),
        Binary::default(), // Empty message
        None,
    )
    .with_config(AMPMsgConfig {
        direct: true,
        reply_on: ReplyOn::Always,
        exit_at_error: false,
        gas_limit: None,
        ibc_config: None,
    });

    let channel_info = ChannelInfo {
        direct_channel_id: Some("channel-direct".to_string()),
        ics20_channel_id: Some("channel-0".to_string()),
        kernel_address: "juno_kernel".to_string(),
        supported_modules: vec![],
    };

    // Execute should fail
    let result = handle_ibc_direct(deps.as_mut(), info, env, None, message, channel_info);

    assert!(result.is_err());
    match result {
        Err(ContractError::InvalidPacket { error }) => {
            assert_eq!(
                error,
                Some("Cannot send an empty message without funds via IBC".to_string())
            );
        }
        _ => panic!("Expected InvalidPacket error"),
    }
}

#[test]
fn test_handle_ibc_direct_no_direct_channel() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let sender = deps.api.addr_make("sender");
    let info = message_info(&sender, &[]);

    // Set up test state
    setup_test_state(&mut deps);

    // Create message
    let message = AMPMsg::new(
        AndrAddr::from_string("ibc://juno-1/recipient".to_string()),
        Binary::from(b"{\"execute_something\":{}}"),
        None,
    )
    .with_config(AMPMsgConfig {
        direct: true,
        reply_on: ReplyOn::Always,
        exit_at_error: false,
        gas_limit: None,
        ibc_config: None,
    });

    // Channel info without direct channel
    let channel_info = ChannelInfo {
        direct_channel_id: None, // Missing direct channel
        ics20_channel_id: Some("channel-0".to_string()),
        kernel_address: "juno_kernel".to_string(),
        supported_modules: vec![],
    };

    // Execute the function and check for the expected error
    let result = handle_ibc_direct(deps.as_mut(), info, env, None, message, channel_info);

    // Assert that the function returns an error
    assert!(result.is_err());

    // Verify it's the correct error type with the expected message
    match result {
        Err(ContractError::InvalidPacket { error }) => {
            assert_eq!(
                error,
                Some("Direct channel not found for chain juno-1".to_string())
            );
        }
        err => panic!("Unexpected error: {:?}", err),
    }
}

#[test]
fn test_handle_ibc_direct_with_existing_context() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let sender = deps.api.addr_make("sender");
    let info = message_info(&sender, &[]);

    // Set up test state
    setup_test_state(&mut deps);

    // Create a message
    let message = AMPMsg::new(
        AndrAddr::from_string("ibc://juno-1/recipient".to_string()),
        Binary::from(b"{\"execute_something\":{}}"),
        None,
    )
    .with_config(AMPMsgConfig {
        direct: true,
        reply_on: ReplyOn::Always,
        exit_at_error: false,
        gas_limit: None,
        ibc_config: None,
    });

    // Create an existing context
    let existing_msg = AMPMsg::new(
        AndrAddr::from_string("original_recipient".to_string()),
        Binary::from(b"{\"original_action\":{}}"),
        None,
    )
    .with_config(AMPMsgConfig {
        direct: true,
        reply_on: ReplyOn::Always,
        exit_at_error: false,
        gas_limit: None,
        ibc_config: None,
    });

    let ctx = Some(AMPPkt::new(
        "origin_account".to_string(),
        "prev_sender".to_string(),
        vec![existing_msg],
    ));

    let channel_info = ChannelInfo {
        direct_channel_id: Some("channel-direct".to_string()),
        ics20_channel_id: Some("channel-0".to_string()),
        kernel_address: "juno_kernel".to_string(),
        supported_modules: vec![],
    };

    // Execute handler with existing context
    let response = handle_ibc_direct(
        deps.as_mut(),
        info,
        env.clone(),
        ctx,
        message.clone(),
        channel_info.clone(),
    )
    .unwrap();

    // Check IBC message with context propagation
    match &response.messages[0].msg {
        CosmosMsg::Ibc(IbcMsg::SendPacket { data, .. }) => {
            let ibc_msg: IbcExecuteMsg = from_json(data).unwrap();
            match ibc_msg {
                IbcExecuteMsg::SendMessage { amp_packet } => {
                    // Should have original origin preserved
                    assert_eq!(amp_packet.ctx.get_origin(), "origin_account");

                    // Should have one hop
                    assert_eq!(amp_packet.ctx.previous_hops.len(), 1);

                    // Verify the recipient is correctly processed
                    assert_eq!(amp_packet.messages[0].recipient.get_raw_path(), "recipient");
                }
                IbcExecuteMsg::SendMessageWithFunds { .. } => {
                    panic!("Expected SendMessage, got SendMessageWithFunds")
                }
                IbcExecuteMsg::CreateADO { .. } => {
                    panic!("Expected SendMessage, got CreateADO")
                }
                IbcExecuteMsg::RegisterUsername { .. } => {
                    panic!("Expected SendMessage, got RegisterUsername")
                }
            }
        }
        _ => panic!("Expected IBC SendPacket message"),
    }
}

#[test]
fn test_handle_ibc_direct_with_complex_path() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let sender = deps.api.addr_make("sender");
    let info = message_info(&sender, &[]);

    // Set up test state
    setup_test_state(&mut deps);

    // Create message with a complex recipient path
    let message = AMPMsg::new(
        "ibc://juno-1/apps/marketplace/listings/123".to_string(),
        Binary::from(b"{\"execute_something\":{}}"),
        None,
    )
    .with_config(AMPMsgConfig {
        direct: true,
        reply_on: ReplyOn::Always,
        exit_at_error: false,
        gas_limit: None,
        ibc_config: None,
    });

    let channel_info = ChannelInfo {
        direct_channel_id: Some("channel-direct".to_string()),
        ics20_channel_id: Some("channel-0".to_string()),
        kernel_address: "juno_kernel".to_string(),
        supported_modules: vec![],
    };

    // Execute handler
    let response = handle_ibc_direct(
        deps.as_mut(),
        info,
        env.clone(),
        None,
        message.clone(),
        channel_info.clone(),
    )
    .unwrap();

    // Check that the complex path is preserved correctly
    match &response.messages[0].msg {
        CosmosMsg::Ibc(IbcMsg::SendPacket { data, .. }) => {
            let ibc_msg: IbcExecuteMsg = from_json(data).unwrap();
            match ibc_msg {
                IbcExecuteMsg::SendMessage { amp_packet } => {
                    // The recipient should have the protocol and chain stripped but preserve the path
                    assert_eq!(
                        amp_packet.messages[0].recipient.get_raw_path(),
                        "/apps/marketplace/listings/123"
                    );
                }
                IbcExecuteMsg::SendMessageWithFunds { .. } => {
                    panic!("Unexpected SendMessageWithFunds");
                }
                IbcExecuteMsg::CreateADO { .. } => {
                    panic!("Unexpected CreateADO");
                }
                IbcExecuteMsg::RegisterUsername { .. } => {
                    panic!("Unexpected RegisterUsername");
                }
            }
        }
        _ => panic!("Expected IBC SendPacket message"),
    }
}

#[test]
fn test_handle_ibc_direct_with_custom_config() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let sender = deps.api.addr_make("sender");
    let info = message_info(&sender, &[]);

    // Set up test state
    setup_test_state(&mut deps);

    // Create message with custom config
    let message = AMPMsg::new(
        AndrAddr::from_string("ibc://juno-1/recipient".to_string()),
        Binary::from(b"{\"execute_something\":{}}"),
        None,
    )
    .with_config(AMPMsgConfig {
        direct: true,
        reply_on: ReplyOn::Always,
        exit_at_error: false,
        gas_limit: None,
        ibc_config: None,
    });

    let channel_info = ChannelInfo {
        direct_channel_id: Some("channel-direct".to_string()),
        ics20_channel_id: Some("channel-0".to_string()),
        kernel_address: "juno_kernel".to_string(),
        supported_modules: vec![],
    };

    // Execute handler
    let response = handle_ibc_direct(
        deps.as_mut(),
        info,
        env.clone(),
        None,
        message.clone(),
        channel_info.clone(),
    )
    .unwrap();

    // Verify that config is preserved
    match &response.messages[0].msg {
        CosmosMsg::Ibc(IbcMsg::SendPacket { data, .. }) => {
            let ibc_msg: IbcExecuteMsg = from_json(data).unwrap();
            match ibc_msg {
                IbcExecuteMsg::SendMessage { amp_packet } => {
                    assert!(amp_packet.messages[0].config.direct);
                    assert_eq!(amp_packet.messages[0].config.reply_on, ReplyOn::Always);
                    assert!(!amp_packet.messages[0].config.exit_at_error);
                    assert_eq!(amp_packet.messages[0].config.gas_limit, None);
                    assert_eq!(amp_packet.messages[0].config.ibc_config, None);
                }
                IbcExecuteMsg::SendMessageWithFunds { .. } => {
                    panic!("Unexpected SendMessageWithFunds");
                }
                IbcExecuteMsg::CreateADO { .. } => {
                    panic!("Unexpected CreateADO");
                }
                IbcExecuteMsg::RegisterUsername { .. } => {
                    panic!("Unexpected RegisterUsername");
                }
            }
        }
        _ => panic!("Expected IBC SendPacket message"),
    }
}

#[test]
fn test_handle_ibc_direct_with_funds_attempt() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let sender = deps.api.addr_make("sender");
    let info = message_info(&sender, &[]);

    // Set up test state
    setup_test_state(&mut deps);

    // Create message with funds (which should be handled by handle_ibc_transfer_funds,
    // but we test handle_ibc_direct in isolation here)
    let message = AMPMsg::new(
        AndrAddr::from_string("ibc://juno-1/recipient".to_string()),
        Binary::from(b"{\"execute_something\":{}}"),
        Some(vec![Coin {
            denom: "uatom".to_string(),
            amount: Uint128::new(100),
        }]),
    )
    .with_config(AMPMsgConfig {
        direct: true,
        reply_on: ReplyOn::Always,
        exit_at_error: false,
        gas_limit: None,
        ibc_config: None,
    });

    let channel_info = ChannelInfo {
        direct_channel_id: Some("channel-direct".to_string()),
        ics20_channel_id: Some("channel-0".to_string()),
        kernel_address: "juno_kernel".to_string(),
        supported_modules: vec![],
    };

    // This should still work in isolation since handle_ibc_direct doesn't check for funds
    let response = handle_ibc_direct(
        deps.as_mut(),
        info,
        env.clone(),
        None,
        message.clone(),
        channel_info.clone(),
    )
    .unwrap();

    // Verify the response (funds are ignored in direct IBC messages)
    assert_eq!(response.messages.len(), 1);
}
