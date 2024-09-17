use andromeda_std::amp::AndrAddr;
use andromeda_std::error::ContractError;
use andromeda_std::os::ibc_registry::{
    AllDenomInfoResponse, DenomInfo, DenomInfoResponse, ExecuteMsg, IBCDenomInfo, InstantiateMsg,
};
use andromeda_std::testing::mock_querier::MOCK_KERNEL_CONTRACT;
use cosmwasm_std::testing::mock_env;
use cosmwasm_std::testing::{mock_dependencies, mock_info};
use cosmwasm_std::Addr;

use crate::contract::{execute, instantiate};
use crate::state::SERVICE_ADDRESS;
use crate::testing::mock::{query_all_denom_info, query_denom_info};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        owner: None,
        kernel_address: Addr::unchecked(MOCK_KERNEL_CONTRACT),
        service_address: AndrAddr::from_string("service_address"),
    };
    let env = mock_env();

    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    assert_eq!(
        SERVICE_ADDRESS.load(deps.as_ref().storage).unwrap(),
        "service_address"
    );
}

#[test]
fn test_store_denom_info() {
    let mut deps = mock_dependencies();
    let info = mock_info("not_service_address", &[]);
    let env = mock_env();
    // Test unauthorized sender
    SERVICE_ADDRESS
        .save(deps.as_mut().storage, &"service_address".to_string())
        .unwrap();
    let denom_info1 = DenomInfo {
        path: "path".to_string(),
        base_denom: "ibc/base_denom".to_string(),
    };

    let denom_info2 = DenomInfo {
        path: "path2".to_string(),
        base_denom: "ibc/base_denom2".to_string(),
    };

    let ibc_denom_info = vec![
        IBCDenomInfo {
            denom: "ibc/denom".to_string(),
            denom_info: denom_info1,
        },
        IBCDenomInfo {
            denom: "ibc/denom".to_string(),
            denom_info: denom_info2,
        },
    ];

    let msg = ExecuteMsg::StoreDenomInfo { ibc_denom_info };

    let err = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // Try with authorized sender but with duplicate denoms in denom info
    let info = mock_info("service_address", &[]);

    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();
    assert_eq!(
        err,
        ContractError::DuplicateDenoms {
            denom: "ibc/denom".to_string()
        }
    );

    // Try with empty denom
    let denom_info2 = DenomInfo {
        path: "path2".to_string(),
        base_denom: "ibc/base_denom2".to_string(),
    };

    let ibc_denom_info = vec![IBCDenomInfo {
        denom: "".to_string(),
        denom_info: denom_info2,
    }];
    let msg = ExecuteMsg::StoreDenomInfo { ibc_denom_info };

    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(err, ContractError::EmptyDenom {});

    // Try with denom that doesn't start with `ibc/`
    let denom_info2 = DenomInfo {
        path: "path2".to_string(),
        base_denom: "ibc/base_denom2".to_string(),
    };

    let ibc_denom_info = vec![IBCDenomInfo {
        denom: "invalid_denom".to_string(),
        denom_info: denom_info2,
    }];
    let msg = ExecuteMsg::StoreDenomInfo { ibc_denom_info };

    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidDenom {
            msg: Some("The denom should start with 'ibc/'".to_string()),
        }
    );

    // Make sure it works
    let denom_info1 = DenomInfo {
        path: "path".to_string(),
        base_denom: "ibc/base_denom".to_string(),
    };

    let denom_info2 = DenomInfo {
        path: "path2".to_string(),
        base_denom: "ibc/base_denom2".to_string(),
    };

    let ibc_denom_info = vec![
        IBCDenomInfo {
            denom: "ibc/denom".to_string(),
            denom_info: denom_info1.clone(),
        },
        IBCDenomInfo {
            denom: "ibc/denom2".to_string(),
            denom_info: denom_info2.clone(),
        },
    ];
    let msg = ExecuteMsg::StoreDenomInfo { ibc_denom_info };

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let denom_query = query_denom_info(deps.as_ref(), "ibc/denom".to_string()).unwrap();
    assert_eq!(
        denom_query,
        DenomInfoResponse {
            denom_info: denom_info1.clone()
        }
    );

    let all_denom_info = query_all_denom_info(deps.as_ref(), None, None).unwrap();
    assert_eq!(
        all_denom_info,
        AllDenomInfoResponse {
            denom_info: vec![denom_info1, denom_info2]
        }
    )
}
