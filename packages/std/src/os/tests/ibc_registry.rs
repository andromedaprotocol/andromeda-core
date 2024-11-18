#[cfg(test)]
mod tests {

    use crate::error::ContractError;
    use crate::os::ibc_registry::*;

    struct ValidateDenomTestCase {
        name: String,
        denom: String,
        expected_res: Result<(), ContractError>,
    }

    #[test]
    fn test_validate_denom() {
        let test_cases = vec![
            ValidateDenomTestCase {
                name: "empty denom".to_string(),
                denom: "".to_string(),
                expected_res: Err(ContractError::InvalidDenom {
                    msg: Some("The denom should start with 'ibc/'".to_string()),
                }),
            },
            ValidateDenomTestCase {
                name: "denom that doesn't start with ibc/".to_string(),
                denom: "random".to_string(),
                expected_res: Err(ContractError::InvalidDenom {
                    msg: Some("The denom should start with 'ibc/'".to_string()),
                }),
            },
            ValidateDenomTestCase {
                name: "Denom with empty ibc/".to_string(),
                denom: "ibc/".to_string(),
                expected_res: Err(ContractError::InvalidDenom {
                    msg: Some("The denom must have exactly 64 characters after 'ibc/'".to_string()),
                }),
            },
            ValidateDenomTestCase {
                name: "Valid denom".to_string(),
                denom: "ibc/eab02686416e4b155cfee9c247171e1c4196b218c6a254f765b0958b3af59d09"
                    .to_string(),
                expected_res: Ok(()),
            },
            ValidateDenomTestCase {
                name: "denom with invalid checksum".to_string(),
                denom: "ibc/1234567890123456789012345678901234567890123456789012345678901234"
                    .to_string(),
                expected_res: Err(ContractError::InvalidDenom {
                    msg: Some("Denom hash does not match. Expected: ibc/eab02686416e4b155cfee9c247171e1c4196b218c6a254f765b0958b3af59d09, Actual: ibc/1234567890123456789012345678901234567890123456789012345678901234".to_string()),
                }),
            },
            ValidateDenomTestCase {
                name: "denom with more than 64 characters after ibc/".to_string(),
                denom: "ibc/12345678901234567890123456789012345678901234567890123456789012345"
                    .to_string(),
                expected_res: Err(ContractError::InvalidDenom {
                    msg: Some("The denom must have exactly 64 characters after 'ibc/'".to_string()),
                }),
            },
        ];

        let default_denom_info = DenomInfo {
            path: "transfer/channel-12/transfer/channel-255".to_string(),
            base_denom: "inj".to_string(),
        };

        for test_case in test_cases {
            let denom = test_case.denom;
            let res = verify_denom(&denom, &default_denom_info);
            assert_eq!(res, test_case.expected_res, "{}", test_case.name);
        }
    }

    struct PathToHopsTestCase {
        name: String,
        path: String,
        expected_res: Result<Vec<Hop>, ContractError>,
    }

    #[test]
    fn test_path_to_hops() {
        let test_cases = vec![
            PathToHopsTestCase {
                name: "unwrap path with 2 hops".to_string(),
                path: "transfer/channel-0/transfer/channel-1".to_string(),
                expected_res: Ok(vec![
                    Hop {
                        port_id: "transfer".to_string(),
                        channel_id: "channel-0".to_string(),
                    },
                    Hop {
                        port_id: "transfer".to_string(),
                        channel_id: "channel-1".to_string(),
                    },
                ]),
            },
            PathToHopsTestCase {
                name: "unwrap path with invalid hops".to_string(),
                path: "transfer/channel-0/transfer".to_string(),
                expected_res: Err(ContractError::InvalidDenomTracePath {
                    path: "transfer/channel-0/transfer".to_string(),
                    msg: Some("Odd number of segments".to_string()),
                }),
            },
            PathToHopsTestCase {
                name: "empty path".to_string(),
                path: "".to_string(),
                expected_res: Ok(vec![]),
            },
            PathToHopsTestCase {
                name: "path with empty port id".to_string(),
                path: "transfer/channel-0//channel-1".to_string(),
                expected_res: Err(ContractError::InvalidDenomTracePath {
                    path: "transfer/channel-0//channel-1".to_string(),
                    msg: Some("Port and channel IDs cannot be empty".to_string()),
                }),
            },
            PathToHopsTestCase {
                name: "path with empty channel id".to_string(),
                path: "transfer//transfer/channel-0".to_string(),
                expected_res: Err(ContractError::InvalidDenomTracePath {
                    path: "transfer//transfer/channel-0".to_string(),
                    msg: Some("Port and channel IDs cannot be empty".to_string()),
                }),
            },
        ];

        for test_case in test_cases {
            let path = test_case.path;
            let res = path_to_hops(path);
            assert_eq!(res, test_case.expected_res, "{}", test_case.name);
        }
    }

    #[test]
    fn test_hops_to_trace() {
        let hops = vec![
            Hop {
                port_id: "transfer".to_string(),
                channel_id: "channel-0".to_string(),
            },
            Hop {
                port_id: "transfer".to_string(),
                channel_id: "channel-1".to_string(),
            },
        ];
        let path = hops_to_path(hops);
        assert_eq!(path, "transfer/channel-0/transfer/channel-1");
    }

    #[test]
    fn test_empty_hops_to_trace() {
        let hops = vec![];
        let path = hops_to_path(hops);
        assert_eq!(path, "");
    }
}
