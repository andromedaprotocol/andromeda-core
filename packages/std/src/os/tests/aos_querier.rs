#[cfg(test)]
mod tests {
    use crate::os::aos_querier::*;
    use crate::{error::ContractError, os::ibc_registry::DenomInfo};

    #[test]
    fn test_get_counterparty_denom() {
        use crate::testing::mock_querier::{
            mock_dependencies_custom, MOCK_ANDR_TO_OSMO_IBC_CHANNEL, MOCK_OSMO_TO_ANDR_IBC_CHANNEL,
        };

        let deps = mock_dependencies_custom(&[]);

        // Test Unwrapping Denom
        let (counterparty_denom, new_denom_info) = AOSQuerier::get_counterparty_denom(
            &deps.as_ref().querier,
            &DenomInfo {
                path: format!("transfer/{MOCK_ANDR_TO_OSMO_IBC_CHANNEL}"),
                base_denom: "uosmo".to_string(),
            },
            MOCK_ANDR_TO_OSMO_IBC_CHANNEL,
        )
        .unwrap();

        assert_eq!(counterparty_denom, "uosmo".to_string());

        let expected_denom_info = DenomInfo::new("uosmo".to_string(), "".to_string());
        assert_eq!(new_denom_info, expected_denom_info);
        assert_eq!(counterparty_denom, "uosmo".to_string());

        // Test Wrapping Denom
        let (counterparty_denom, new_denom_info) = AOSQuerier::get_counterparty_denom(
            &deps.as_ref().querier,
            &DenomInfo {
                path: "".to_string(),
                base_denom: "uandr".to_string(),
            },
            MOCK_ANDR_TO_OSMO_IBC_CHANNEL,
        )
        .unwrap();

        let expected_denom_info = DenomInfo::new(
            "uandr".to_string(),
            format!("transfer/{MOCK_OSMO_TO_ANDR_IBC_CHANNEL}"),
        );
        assert_eq!(new_denom_info, expected_denom_info);
        assert_eq!(counterparty_denom, expected_denom_info.get_ibc_denom());

        // Test Multi Hop Wrapping Denom
        let (counterparty_denom, new_denom_info) = AOSQuerier::get_counterparty_denom(
            &deps.as_ref().querier,
            &DenomInfo {
                path: "transfer/channel-13".to_string(),
                base_denom: "testdenom".to_string(),
            },
            MOCK_ANDR_TO_OSMO_IBC_CHANNEL,
        )
        .unwrap();

        let expected_denom_info = DenomInfo::new(
            "testdenom".to_string(),
            format!("transfer/channel-13/transfer/{MOCK_OSMO_TO_ANDR_IBC_CHANNEL}"),
        );
        assert_eq!(new_denom_info, expected_denom_info);
        assert_eq!(counterparty_denom, expected_denom_info.get_ibc_denom());

        // Test Multi Hop UnWrapping Denom
        let (counterparty_denom, new_denom_info) = AOSQuerier::get_counterparty_denom(
            &deps.as_ref().querier,
            &DenomInfo {
                path: "transfer/channel-13/transfer/channel-0".to_string(),
                base_denom: "testdenom".to_string(),
            },
            MOCK_ANDR_TO_OSMO_IBC_CHANNEL,
        )
        .unwrap();

        let expected_denom_info =
            DenomInfo::new("testdenom".to_string(), "transfer/channel-13".to_string());
        assert_eq!(new_denom_info, expected_denom_info);
        assert_eq!(counterparty_denom, expected_denom_info.get_ibc_denom());

        // Test invalid channel
        let err = AOSQuerier::get_counterparty_denom(
            &deps.as_ref().querier,
            &DenomInfo {
                path: "".to_string(),
                base_denom: "uandr".to_string(),
            },
            "channel-13", // This channel does not exist
        )
        .unwrap_err();

        assert_eq!(
            err,
            ContractError::InvalidDenomTracePath {
                path: "".to_string(),
                msg: Some("Channel info not found".to_string()),
            }
        );
    }
}
