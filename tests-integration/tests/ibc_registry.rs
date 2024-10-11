use andromeda_app_contract::mock::mock_andromeda_app;
use andromeda_ibc_registry::mock::mock_andromeda_ibc_registry;
use andromeda_std::{
    error::ContractError,
    os::ibc_registry::{AllDenomInfoResponse, DenomInfo, DenomInfoResponse, IBCDenomInfo},
};
use andromeda_testing::{
    ibc_registry::MockIbcRegistry, mock::mock_app, mock_builder::MockAndromedaBuilder,
};
use cosmwasm_std::coin;

#[test]
fn test_ibc_registry() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![coin(100_000, "uandr"), coin(100_000, "uusd")]),
            ("service_address", vec![]),
            ("user1", vec![]),
        ])
        .with_contracts(vec![
            ("app-contract", mock_andromeda_app()),
            ("ibc_registry", mock_andromeda_ibc_registry()),
        ])
        .build(&mut router);
    let service_address = andr.get_wallet("service_address").clone();
    let user1 = andr.get_wallet("user1").clone();

    let ibc_registry: MockIbcRegistry = andr.ibc_registry;

    // Test Store Denom Info
    let denom_info = DenomInfo {
        path: "path".to_string(),
        base_denom: "base_denom".to_string(),
    };
    let denom = denom_info.get_ibc_denom();
    let ibc_denom_info = IBCDenomInfo {
        denom: denom.clone(),
        denom_info,
    };
    ibc_registry
        .execute_execute_store_denom_info(
            &mut router,
            service_address.clone(),
            None,
            vec![ibc_denom_info],
        )
        .unwrap();

    let query_res = ibc_registry.query_denom_info(&mut router, denom);
    assert_eq!(
        query_res,
        DenomInfoResponse {
            denom_info: DenomInfo {
                path: "path".to_string(),
                base_denom: "base_denom".to_string(),
            }
        }
    );

    // Store one more denom
    let denom_info = DenomInfo {
        path: "path2".to_string(),
        base_denom: "base_denom2".to_string(),
    };
    let denom = denom_info.get_ibc_denom();
    let ibc_denom_info = IBCDenomInfo {
        denom: denom.clone(),
        denom_info,
    };
    ibc_registry
        .execute_execute_store_denom_info(
            &mut router,
            service_address.clone(),
            None,
            vec![ibc_denom_info],
        )
        .unwrap();

    // Query all denoms
    let query_res = ibc_registry.query_all_denom_info(&mut router, None, None);
    assert_eq!(
        query_res,
        AllDenomInfoResponse {
            denom_info: vec![
                DenomInfo {
                    path: "path".to_string(),
                    base_denom: "base_denom".to_string(),
                },
                DenomInfo {
                    path: "path2".to_string(),
                    base_denom: "base_denom2".to_string(),
                },
            ]
        }
    );

    // Test real data
    let path = "transfer/channel-12/transfer/channel-255".to_string();
    let base_denom = "inj".to_string();

    let denom_info = DenomInfo { path, base_denom };
    assert_eq!(
        denom_info.get_ibc_denom(),
        "ibc/eab02686416e4b155cfee9c247171e1c4196b218c6a254f765b0958b3af59d09".to_string()
    );

    // Test authorization
    let ibc_denom_info = IBCDenomInfo {
        denom: "ibc/usdcusdcusdcusdcusdcusdcusdcusdcusdcusdcusdcusdcusdcusdcusdcusdt".to_string(),
        denom_info: DenomInfo {
            path: "path3".to_string(),
            base_denom: "base_denom3".to_string(),
        },
    };
    let err: ContractError = ibc_registry
        .execute_execute_store_denom_info(&mut router, user1.clone(), None, vec![ibc_denom_info])
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});
}
