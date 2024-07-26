use andromeda_modules::time_gate::{
    GateAddresses, GateTime, 
};
use andromeda_std::amp::AndrAddr;
use andromeda_std::error::ContractError;
use super::mock::{
    proper_initialization, set_gate_time, update_gate_addresses, query_gate_addresses, query_gate_time, query_path,
};

#[test]
fn test_instantiation() {
    let (deps, _) = proper_initialization(
        GateAddresses { 
            ado_1: AndrAddr::from_string("mock_ado_1".to_string()), 
            ado_2: AndrAddr::from_string("mock_ado_2".to_string())
        },
        GateTime { 
            year: 1999, 
            month: 2, 
            day: 28, 
            hour: 0, 
            minute: 0, 
            second: 0, 
        }
    );

    let res = query_gate_addresses(deps.as_ref()).unwrap();
    assert_eq!(res, GateAddresses { 
        ado_1: AndrAddr::from_string("mock_ado_1".to_string()), 
        ado_2: AndrAddr::from_string("mock_ado_2".to_string())
    });

    let res = query_gate_time(deps.as_ref()).unwrap();
    assert_eq!(
        res,
        GateTime { 
            year: 1999, 
            month: 2, 
            day: 28, 
            hour: 0, 
            minute: 0, 
            second: 0, 
        }
    );
}

#[test]
fn test_set_gate_time() {
    let (mut deps, info) = proper_initialization(
        GateAddresses { 
            ado_1: AndrAddr::from_string("mock_ado_1".to_string()), 
            ado_2: AndrAddr::from_string("mock_ado_2".to_string())
        },
        GateTime { 
            year: 2024, 
            month: 1, 
            day: 1, 
            hour: 0, 
            minute: 0, 
            second: 0, 
        }
    );

    let err_res = set_gate_time(
        deps.as_mut(), 
        GateTime { 
            year: 2024, 
            month: 1, 
            day: 1, 
            hour: 0, 
            minute: 0, 
            second: 0, 
        },
        info.sender.as_ref(),
    ).unwrap_err();

    assert_eq!(err_res, ContractError::InvalidParameter { error: Some("Same as existed gate time".to_string())});

    set_gate_time(
        deps.as_mut(), 
        GateTime { 
            year: 2023, 
            month: 1, 
            day: 1, 
            hour: 0, 
            minute: 0, 
            second: 0, 
        },
        info.sender.as_ref(),
    ).unwrap();

    let res = query_gate_time(deps.as_ref()).unwrap();
    assert_eq!(
        res,
        GateTime { 
            year: 2023, 
            month: 1, 
            day: 1, 
            hour: 0, 
            minute: 0, 
            second: 0, 
        }
    );
}

#[test]
fn test_update_gate_addresses() {
    let (mut deps, info) = proper_initialization(
        GateAddresses { 
            ado_1: AndrAddr::from_string("mock_ado_1".to_string()), 
            ado_2: AndrAddr::from_string("mock_ado_2".to_string())
        },
        GateTime { 
            year: 2024, 
            month: 1, 
            day: 1, 
            hour: 0, 
            minute: 0, 
            second: 0, 
        }
    );

    let err_res = update_gate_addresses(
        deps.as_mut(), 
        GateAddresses { 
            ado_1: AndrAddr::from_string("mock_ado_1".to_string()), 
            ado_2: AndrAddr::from_string("mock_ado_2".to_string())
        },
        info.sender.as_ref(),
    ).unwrap_err();

    assert_eq!(err_res, ContractError::InvalidParameter { error: Some("Same as existed gate addresses".to_string())});

    update_gate_addresses(
        deps.as_mut(), 
        GateAddresses { 
            ado_1: AndrAddr::from_string("mock_ado_2".to_string()), 
            ado_2: AndrAddr::from_string("mock_ado_3".to_string())
        },
        info.sender.as_ref(),
    ).unwrap();

    let res = query_gate_addresses(deps.as_ref()).unwrap();
    assert_eq!(res, GateAddresses { 
        ado_1: AndrAddr::from_string("mock_ado_2".to_string()), 
        ado_2: AndrAddr::from_string("mock_ado_3".to_string())
    });
}

#[test]
fn test_query_path() {
    let (deps, _) = proper_initialization(
        GateAddresses { 
            ado_1: AndrAddr::from_string("mock_ado_1".to_string()), 
            ado_2: AndrAddr::from_string("mock_ado_2".to_string())
        },
        GateTime { 
            year: 2019, 
            month: 10, 
            day: 22, 
            hour: 0, 
            minute: 0, 
            second: 0, 
        }
    );

    let res = query_path(deps.as_ref()).unwrap();
    assert_eq!(res.to_string(), "mock_ado_1".to_string());
}
