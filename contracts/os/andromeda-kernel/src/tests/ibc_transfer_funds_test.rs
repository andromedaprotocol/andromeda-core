#[cfg(test)]
mod ibc_transfer_tests {
    use crate::execute::handle_ibc_transfer_funds;
    use crate::state::PENDING_MSG_AND_FUNDS;
    use andromeda_std::amp::messages::{AMPMsg, AMPMsgConfig, AMPPkt};
    use andromeda_std::amp::AndrAddr;
    use andromeda_std::common::reply::ReplyId;
    use andromeda_std::error::ContractError;
    use andromeda_std::os::kernel::ChannelInfo;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{Binary, Coin, CosmosMsg, IbcMsg, SubMsg, Uint128};

    #[test]
    fn test_handle_ibc_transfer_funds_success() {
        // Setup
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("sender", &[]);

        // Create message with funds
        let message = AMPMsg {
            recipient: AndrAddr::from_string("ibc://juno-1/recipient".to_string()),
            message: Binary::from(b"{\"execute_something\":{}}"),
            funds: vec![Coin {
                denom: "uatom".to_string(),
                amount: Uint128::new(100),
            }],
            config: AMPMsgConfig::default(),
        };

        // Create channel info
        let channel_info = ChannelInfo {
            direct_channel_id: Some("channel-direct".to_string()),
            ics20_channel_id: Some("channel-0".to_string()),
            kernel_address: "juno_kernel".to_string(),
            supported_modules: vec![],
        };

        // Execute handler
        let response = handle_ibc_transfer_funds(
            deps.as_mut(),
            info,
            env.clone(),
            None,
            message.clone(),
            channel_info.clone(),
        )
        .unwrap();

        // Check submessage
        assert_eq!(response.messages.len(), 1);
        match &response.messages[0] {
            SubMsg { id, msg, .. } => {
                assert_eq!(*id, ReplyId::IBCTransfer.repr());
                match msg {
                    CosmosMsg::Ibc(IbcMsg::Transfer {
                        channel_id,
                        to_address,
                        amount,
                        ..
                    }) => {
                        assert_eq!(channel_id, "channel-0");
                        assert_eq!(to_address, "juno_kernel");
                        assert_eq!(amount.denom, "uatom");
                        assert_eq!(amount.amount, Uint128::new(100));
                    }
                    _ => panic!("Expected IBC Transfer message"),
                }
            }
        }

        // Check attributes
        assert!(response
            .attributes
            .iter()
            .any(|attr| attr.key == "method" && attr.value == "execute_transfer_funds"));
        assert!(response
            .attributes
            .iter()
            .any(|attr| attr.key == "channel" && attr.value == "channel-0"));

        // Check storage
        let stored_packet = PENDING_MSG_AND_FUNDS.load(&deps.storage).unwrap();
        assert_eq!(stored_packet.sender, "sender");
        assert_eq!(stored_packet.channel, "channel-0");
        assert_eq!(stored_packet.funds.amount, Uint128::new(100));
        assert_eq!(stored_packet.funds.denom, "uatom");
    }

    #[test]
    fn test_handle_ibc_transfer_funds_no_ics20_channel() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("sender", &[]);

        // Create message
        let message = AMPMsg {
            recipient: AndrAddr::from_string("ibc://juno-1/recipient".to_string()),
            message: Binary::default(),
            funds: vec![Coin {
                denom: "uatom".to_string(),
                amount: Uint128::new(100),
            }],
            config: AMPMsgConfig::default(),
        };

        // Channel info without ICS20 channel
        let channel_info = ChannelInfo {
            direct_channel_id: Some("channel-direct".to_string()),
            ics20_channel_id: None, // Missing ICS20 channel
            kernel_address: "juno_kernel".to_string(),
            supported_modules: vec![],
        };

        // Execute should fail
        let result =
            handle_ibc_transfer_funds(deps.as_mut(), info, env, None, message, channel_info);

        assert!(result.is_err());
        match result {
            Err(ContractError::InvalidPacket { error }) => {
                assert_eq!(
                    error,
                    Some("Channel not found for chain juno-1".to_string())
                );
            }
            _ => panic!("Expected InvalidPacket error"),
        }
    }

    #[test]
    fn test_handle_ibc_transfer_funds_multiple_coins() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("sender", &[]);

        // Create message with multiple coins
        let message = AMPMsg {
            recipient: AndrAddr::from_string("ibc://juno-1/recipient".to_string()),
            message: Binary::default(),
            funds: vec![
                Coin {
                    denom: "uatom".to_string(),
                    amount: Uint128::new(100),
                },
                Coin {
                    denom: "usdc".to_string(),
                    amount: Uint128::new(50),
                },
            ],
            config: AMPMsgConfig::default(),
        };

        let channel_info = ChannelInfo {
            direct_channel_id: Some("channel-direct".to_string()),
            ics20_channel_id: Some("channel-0".to_string()),
            kernel_address: "juno_kernel".to_string(),
            supported_modules: vec![],
        };

        // Execute should fail
        let result =
            handle_ibc_transfer_funds(deps.as_mut(), info, env, None, message, channel_info);

        assert!(result.is_err());
        match result {
            Err(ContractError::InvalidFunds { msg }) => {
                assert_eq!(msg, "Number of funds should be exactly one".to_string());
            }
            _ => panic!("Expected InvalidFunds error"),
        }
    }

    #[test]
    fn test_handle_ibc_transfer_funds_empty_funds() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("sender", &[]);

        // Create message with empty funds
        let message = AMPMsg {
            recipient: AndrAddr::from_string("ibc://juno-1/recipient".to_string()),
            message: Binary::default(),
            funds: vec![],
            config: AMPMsgConfig::default(),
        };

        let channel_info = ChannelInfo {
            direct_channel_id: Some("channel-direct".to_string()),
            ics20_channel_id: Some("channel-0".to_string()),
            kernel_address: "juno_kernel".to_string(),
            supported_modules: vec![],
        };

        // Execute should fail
        let result =
            handle_ibc_transfer_funds(deps.as_mut(), info, env, None, message, channel_info);

        assert!(result.is_err());
        match result {
            Err(ContractError::InvalidFunds { msg }) => {
                assert_eq!(msg, "Number of funds should be exactly one".to_string());
            }
            _ => panic!("Expected InvalidFunds error"),
        }
    }

    #[test]
    fn test_handle_ibc_transfer_funds_no_chain_in_recipient() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("sender", &[]);

        // Create message with no chain in recipient
        let message = AMPMsg {
            recipient: AndrAddr::from_string("recipient".to_string()), // No chain prefix
            message: Binary::default(),
            funds: vec![Coin {
                denom: "uatom".to_string(),
                amount: Uint128::new(100),
            }],
            config: AMPMsgConfig::default(),
        };

        let channel_info = ChannelInfo {
            direct_channel_id: Some("channel-direct".to_string()),
            ics20_channel_id: Some("channel-0".to_string()),
            kernel_address: "juno_kernel".to_string(),
            supported_modules: vec![],
        };

        // Execute should fail
        let result =
            handle_ibc_transfer_funds(deps.as_mut(), info, env, None, message, channel_info);

        assert!(result.is_err());
        match result {
            Err(ContractError::InvalidPacket { error }) => {
                assert_eq!(error, Some("Chain not provided in recipient".to_string()));
            }
            _ => panic!("Expected InvalidPacket error"),
        }
    }

    #[test]
    fn test_handle_ibc_transfer_funds_response_attributes() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("sender", &[]);

        // Create message
        let message = AMPMsg {
            recipient: AndrAddr::from_string("ibc://cosmos-hub/recipient".to_string()),
            message: Binary::from(b"{\"execute_something\":{}}"),
            funds: vec![Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::new(500),
            }],
            config: AMPMsgConfig::default(),
        };

        let channel_info = ChannelInfo {
            direct_channel_id: Some("channel-3".to_string()),
            ics20_channel_id: Some("channel-5".to_string()),
            kernel_address: "cosmos_kernel".to_string(),
            supported_modules: vec![],
        };

        // Execute handler
        let response = handle_ibc_transfer_funds(
            deps.as_mut(),
            info,
            env.clone(),
            None,
            message.clone(),
            channel_info.clone(),
        )
        .unwrap();

        // Verify all attributes are present with correct values
        let attributes = response.attributes;
        assert!(attributes
            .iter()
            .any(|attr| attr.key == "method" && attr.value == "execute_transfer_funds"));
        assert!(attributes
            .iter()
            .any(|attr| attr.key == "channel" && attr.value == "channel-5"));
        assert!(
            attributes
                .iter()
                .any(|attr| attr.key == "receiving_kernel_address:{}"
                    && attr.value == "cosmos_kernel")
        );
        assert!(attributes
            .iter()
            .any(|attr| attr.key == "chain:{}" && attr.value == "cosmos-hub"));
    }

    #[test]
    fn test_handle_ibc_transfer_funds_custom_denom() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("sender", &[]);

        // Create message with custom denom
        let message = AMPMsg {
            recipient: AndrAddr::from_string("ibc://juno-1/recipient".to_string()),
            message: Binary::default(),
            funds: vec![Coin {
                denom: "factory/contract/token".to_string(), // Custom token format
                amount: Uint128::new(100),
            }],
            config: AMPMsgConfig::default(),
        };

        let channel_info = ChannelInfo {
            direct_channel_id: Some("channel-direct".to_string()),
            ics20_channel_id: Some("channel-0".to_string()),
            kernel_address: "juno_kernel".to_string(),
            supported_modules: vec![],
        };

        // Execute handler
        let response = handle_ibc_transfer_funds(
            deps.as_mut(),
            info,
            env.clone(),
            None,
            message.clone(),
            channel_info.clone(),
        )
        .unwrap();

        // Check IBC transfer message
        match &response.messages[0].msg {
            CosmosMsg::Ibc(IbcMsg::Transfer { amount, .. }) => {
                assert_eq!(amount.denom, "factory/contract/token");
                assert_eq!(amount.amount, Uint128::new(100));
            }
            _ => panic!("Expected IBC Transfer message"),
        }

        // Check storage
        let stored_packet = PENDING_MSG_AND_FUNDS.load(&deps.storage).unwrap();
        assert_eq!(stored_packet.funds.denom, "factory/contract/token");
    }

    #[test]
    fn test_handle_ibc_transfer_funds_with_empty_message() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("sender", &[]);

        // Create message with empty binary message (valid case)
        let message = AMPMsg {
            recipient: AndrAddr::from_string("ibc://juno-1/recipient".to_string()),
            message: Binary::default(), // Empty message
            funds: vec![Coin {
                denom: "uatom".to_string(),
                amount: Uint128::new(100),
            }],
            config: AMPMsgConfig::default(),
        };

        let channel_info = ChannelInfo {
            direct_channel_id: Some("channel-direct".to_string()),
            ics20_channel_id: Some("channel-0".to_string()),
            kernel_address: "juno_kernel".to_string(),
            supported_modules: vec![],
        };

        // Execute handler - should succeed as empty message is valid for fund transfers
        let _response = handle_ibc_transfer_funds(
            deps.as_mut(),
            info,
            env.clone(),
            None,
            message.clone(),
            channel_info.clone(),
        )
        .unwrap();

        // Verify stored packet
        let stored_packet = PENDING_MSG_AND_FUNDS.load(&deps.storage).unwrap();
        assert_eq!(stored_packet.message, Binary::default());
    }

    #[test]
    fn test_handle_ibc_transfer_funds_with_different_senders() {
        // Test with different sender addresses
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("another_sender", &[]); // Different sender

        let message = AMPMsg {
            recipient: AndrAddr::from_string("ibc://juno-1/recipient".to_string()),
            message: Binary::from(b"{\"execute_something\":{}}"),
            funds: vec![Coin {
                denom: "uatom".to_string(),
                amount: Uint128::new(100),
            }],
            config: AMPMsgConfig::default(),
        };

        let channel_info = ChannelInfo {
            direct_channel_id: Some("channel-direct".to_string()),
            ics20_channel_id: Some("channel-0".to_string()),
            kernel_address: "juno_kernel".to_string(),
            supported_modules: vec![],
        };

        // Execute handler
        let _response = handle_ibc_transfer_funds(
            deps.as_mut(),
            info,
            env.clone(),
            None,
            message.clone(),
            channel_info.clone(),
        )
        .unwrap();

        // Verify stored packet has the correct sender
        let stored_packet = PENDING_MSG_AND_FUNDS.load(&deps.storage).unwrap();
        assert_eq!(stored_packet.sender, "another_sender");
    }

    #[test]
    fn test_handle_ibc_transfer_funds_context_ignored() {
        // Test that context is ignored as specified in the function signature
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("sender", &[]);

        // Create a dummy context
        let ctx = Some(AMPPkt::new(
            "origin".to_string(),
            "prev_sender".to_string(),
            vec![],
        ));

        let message = AMPMsg {
            recipient: AndrAddr::from_string("ibc://juno-1/recipient".to_string()),
            message: Binary::from(b"{\"execute_something\":{}}"),
            funds: vec![Coin {
                denom: "uatom".to_string(),
                amount: Uint128::new(100),
            }],
            config: AMPMsgConfig::default(),
        };

        let channel_info = ChannelInfo {
            direct_channel_id: Some("channel-direct".to_string()),
            ics20_channel_id: Some("channel-0".to_string()),
            kernel_address: "juno_kernel".to_string(),
            supported_modules: vec![],
        };

        // Execute handler with context
        let _response = handle_ibc_transfer_funds(
            deps.as_mut(),
            info,
            env.clone(),
            ctx,
            message.clone(),
            channel_info.clone(),
        )
        .unwrap();

        // Function should succeed even with context (which is ignored)
        let stored_packet = PENDING_MSG_AND_FUNDS.load(&deps.storage).unwrap();
        assert_eq!(stored_packet.sender, "sender"); // Original sender preserved
    }
}
