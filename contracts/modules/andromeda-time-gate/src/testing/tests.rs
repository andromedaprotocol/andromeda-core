use super::mock::{
    proper_initialization, query_current_ado_path, query_cycle_start_time, query_gate_addresses,
    query_time_interval, update_cycle_start_time, update_gate_addresses, update_time_interval,
};
use andromeda_modules::time_gate::CycleStartTime;
use andromeda_std::amp::AndrAddr;
use andromeda_std::error::ContractError;
use cosmwasm_std::testing::mock_env;

#[test]
fn test_instantiation() {
    let (deps, _) = proper_initialization(
        vec![
            AndrAddr::from_string("mock_ado_1".to_string()),
            AndrAddr::from_string("mock_ado_2".to_string()),
            AndrAddr::from_string("mock_ado_3".to_string()),
        ],
        CycleStartTime {
            year: 2022,
            month: 2,
            day: 28,
            hour: 0,
            minute: 0,
            second: 0,
        },
        None,
    );

    let res = query_gate_addresses(deps.as_ref()).unwrap();
    assert_eq!(
        res,
        vec![
            AndrAddr::from_string("mock_ado_1".to_string()),
            AndrAddr::from_string("mock_ado_2".to_string()),
            AndrAddr::from_string("mock_ado_3".to_string()),
        ]
    );

    let res = query_cycle_start_time(deps.as_ref()).unwrap();
    assert_eq!(
        res,
        CycleStartTime {
            year: 2022,
            month: 2,
            day: 28,
            hour: 0,
            minute: 0,
            second: 0,
        }
    );

    let res = query_time_interval(deps.as_ref()).unwrap();
    assert_eq!(res, "3600".to_string());
}

#[test]
fn test_update_cycle_start_time() {
    let (mut deps, info) = proper_initialization(
        vec![
            AndrAddr::from_string("mock_ado_1".to_string()),
            AndrAddr::from_string("mock_ado_2".to_string()),
            AndrAddr::from_string("mock_ado_3".to_string()),
        ],
        CycleStartTime {
            year: 2022,
            month: 2,
            day: 28,
            hour: 0,
            minute: 0,
            second: 0,
        },
        None,
    );

    let err_res = update_cycle_start_time(
        deps.as_mut(),
        CycleStartTime {
            year: 2022,
            month: 2,
            day: 28,
            hour: 0,
            minute: 0,
            second: 0,
        },
        info.sender.as_ref(),
    )
    .unwrap_err();

    assert_eq!(
        err_res,
        ContractError::InvalidParameter {
            error: Some("Same as an existed cycle start time".to_string())
        }
    );

    update_cycle_start_time(
        deps.as_mut(),
        CycleStartTime {
            year: 2023,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
        },
        info.sender.as_ref(),
    )
    .unwrap();

    let res = query_cycle_start_time(deps.as_ref()).unwrap();
    assert_eq!(
        res,
        CycleStartTime {
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
        vec![
            AndrAddr::from_string("mock_ado_1".to_string()),
            AndrAddr::from_string("mock_ado_2".to_string()),
            AndrAddr::from_string("mock_ado_3".to_string()),
        ],
        CycleStartTime {
            year: 2024,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
        },
        None,
    );

    let err_res = update_gate_addresses(
        deps.as_mut(),
        vec![
            AndrAddr::from_string("mock_ado_1".to_string()),
            AndrAddr::from_string("mock_ado_2".to_string()),
            AndrAddr::from_string("mock_ado_3".to_string()),
        ],
        info.sender.as_ref(),
    )
    .unwrap_err();

    assert_eq!(
        err_res,
        ContractError::InvalidParameter {
            error: Some("Same as existed gate addresses".to_string())
        }
    );

    update_gate_addresses(
        deps.as_mut(),
        vec![
            AndrAddr::from_string("mock_ado_1".to_string()),
            AndrAddr::from_string("mock_ado_2".to_string()),
            AndrAddr::from_string("mock_ado_3".to_string()),
            AndrAddr::from_string("mock_ado_4".to_string()),
        ],
        info.sender.as_ref(),
    )
    .unwrap();

    let res = query_gate_addresses(deps.as_ref()).unwrap();
    assert_eq!(
        res,
        vec![
            AndrAddr::from_string("mock_ado_1".to_string()),
            AndrAddr::from_string("mock_ado_2".to_string()),
            AndrAddr::from_string("mock_ado_3".to_string()),
            AndrAddr::from_string("mock_ado_4".to_string()),
        ]
    );
}

#[test]
fn test_query_current_ado_path_not_started_yet() {
    let (deps, _) = proper_initialization(
        vec![
            AndrAddr::from_string("mock_ado_1".to_string()),
            AndrAddr::from_string("mock_ado_2".to_string()),
            AndrAddr::from_string("mock_ado_3".to_string()),
        ],
        CycleStartTime {
            year: 2024,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
        },
        None,
    );

    let env = mock_env();
    let res = query_current_ado_path(deps.as_ref(), env).unwrap_err();
    assert_eq!(
        res,
        ContractError::CustomError {
            msg: "Cycle is not started yet".to_string()
        }
    );
}

#[test]
fn test_query_current_ado_path() {
    let (deps, _) = proper_initialization(
        vec![
            AndrAddr::from_string("mock_ado_1".to_string()),
            AndrAddr::from_string("mock_ado_2".to_string()),
            AndrAddr::from_string("mock_ado_3".to_string()),
            AndrAddr::from_string("mock_ado_4".to_string()),
            AndrAddr::from_string("mock_ado_5".to_string()),
        ],
        CycleStartTime {
            year: 2019,
            month: 10,
            day: 23,
            hour: 1,
            minute: 30,
            second: 0,
        },
        None,
    );

    let mut env = mock_env();
    let res = query_current_ado_path(deps.as_ref(), env.clone()).unwrap();
    assert_eq!(res, "mock_ado_1".to_string());

    env.block.time = env.block.time.plus_seconds(3600);
    let res = query_current_ado_path(deps.as_ref(), env.clone()).unwrap();
    assert_eq!(res, "mock_ado_2".to_string());

    env.block.time = env.block.time.plus_seconds(3600 * 3);
    let res = query_current_ado_path(deps.as_ref(), env.clone()).unwrap();
    assert_eq!(res, "mock_ado_5".to_string());

    env.block.time = env.block.time.plus_seconds(3600);
    let res = query_current_ado_path(deps.as_ref(), env.clone()).unwrap();
    assert_eq!(res, "mock_ado_1".to_string());
}
