use super::mock::{
    close_form, invalid_initialization, open_form, query_all_submissions, query_form_status,
    query_schema, query_submission, valid_initialization,
};
use andromeda_data_storage::form::{
    FormConfig, GetFormStatusResponse, GetSchemaResponse, SubmissionInfo,
};
use andromeda_std::{
    amp::AndrAddr,
    common::{expiration::Expiry, Milliseconds},
    error::ContractError,
};
use cosmwasm_std::{testing::mock_env, Addr, Timestamp};
use test_case::test_case;

use crate::{
    state::{END_TIME, START_TIME},
    testing::mock::{delete_submission, edit_submission, submit_form},
};

pub const MOCK_SCHEMA_ADO: &str = "schema_ado";

#[test_case(
    FormConfig {
        start_time: None,
        end_time: None,
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    10000_u64;
    "With none start and none end time"
)]
#[test_case(
    FormConfig {
        start_time: Some(Expiry::AtTime(Milliseconds::from_nanos(1000002000000_u64))),
        end_time: None,
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    1000000000000_u64;
    "With valid start time and none end time"
)]
#[test_case(
    FormConfig {
        start_time: None,
        end_time: Some(Expiry::AtTime(Milliseconds::from_nanos(1000002000000_u64))),
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    1000000000000_u64;
    "With none start time and valid end time"
)]
#[test_case(
    FormConfig {
        start_time: Some(Expiry::AtTime(Milliseconds::from_nanos(1000002000000_u64))),
        end_time: Some(Expiry::AtTime(Milliseconds::from_nanos(2000000000000_u64))),
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    1000000000000_u64;
    "With valid start and end time"
)]
#[test_case(
    FormConfig {
        start_time: Some(Expiry::FromNow(Milliseconds::from_nanos(1000002000000_u64))),
        end_time: None,
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    1000000000000_u64;
    "With valid FromNow start time and none end time"
)]
#[test_case(
    FormConfig {
        start_time: None,
        end_time: Some(Expiry::FromNow(Milliseconds::from_nanos(1000002000000_u64))),
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    1000000000000_u64;
    "With none start time and valid FromNow end time"
)]
#[test_case(
    FormConfig {
        start_time: Some(Expiry::FromNow(Milliseconds::from_nanos(1000000000000_u64))),
        end_time: Some(Expiry::FromNow(Milliseconds::from_nanos(2000000000000_u64))),
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    1000000000000_u64;
    "With valid FromNow start and end time"
)]
fn test_valid_instantiation(form_config: FormConfig, timestamp: u64) {
    valid_initialization(
        AndrAddr::from_string(MOCK_SCHEMA_ADO),
        None,
        form_config,
        None,
        timestamp,
    );
}

#[test_case(
    FormConfig {
        start_time: Some(Expiry::AtTime(Milliseconds::from_nanos(900000000000_u64))),
        end_time: None,
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    1000000000000_u64,
    ContractError::StartTimeInThePast { current_time: 1000000_u64, current_block: 12345_u64 };
    "With invalid start and none end time"
)]
#[test_case(
    FormConfig {
        start_time: None,
        end_time: Some(Expiry::AtTime(Milliseconds::from_nanos(900000000000_u64))),
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    1000000000000_u64,
    ContractError::CustomError {
        msg: format!(
            "End time in the past. current_time {:?}, current_block {:?}",
            1000000_u64, 12345_u64
        ),
    };
    "With none start and invalid end time"
)]
#[test_case(
    FormConfig {
        start_time: Some(Expiry::AtTime(Milliseconds::from_nanos(2000000000000_u64))),
        end_time: Some(Expiry::AtTime(Milliseconds::from_nanos(1200000000000_u64))),
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    1000000000000_u64,
    ContractError::StartTimeAfterEndTime {};
    "With invalid start and end time_1"
)]
#[test_case(
    FormConfig {
        start_time: Some(Expiry::FromNow(Milliseconds::from_nanos(2000000000000_u64))),
        end_time: Some(Expiry::FromNow(Milliseconds::from_nanos(1200000000000_u64))),
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    1000000000000_u64,
    ContractError::StartTimeAfterEndTime {};
    "With invalid start and end time_2"
)]
fn test_invalid_instantiation(
    form_config: FormConfig,
    timestamp: u64,
    expected_err: ContractError,
) {
    let (_, _, err) = invalid_initialization(
        AndrAddr::from_string(MOCK_SCHEMA_ADO),
        None,
        form_config,
        None,
        timestamp,
    );
    assert_eq!(expected_err, err);
}

#[test_case(
    FormConfig {
        start_time: Some(Expiry::AtTime(Milliseconds::from_nanos(1000000000000_u64))),
        end_time: None,
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    500000000000_u64,
    2000000000000_u64,
    ContractError::CustomError {
        msg: format!("Already opened. Opened time {:?}", Milliseconds::from_nanos(1000000000000_u64)),
    };
    "Invalid timestamp at execution with saved start time"
)]
#[test_case(
    FormConfig {
        start_time: Some(Expiry::AtTime(Milliseconds::from_nanos(1000000000000_u64))),
        end_time: Some(Expiry::AtTime(Milliseconds::from_nanos(3000000000000_u64))),
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    500000000000_u64,
    2000000000000_u64,
    ContractError::CustomError {
        msg: format!("Already opened. Opened time {:?}", Milliseconds::from_nanos(1000000000000_u64)),
    };
    "Invalid timestamp at execution with saved start and end time"
)]
fn test_failed_open_form(
    form_config: FormConfig,
    instantiation_timestamp: u64,
    execute_timestamp: u64,
    expected_err: ContractError,
) {
    let (mut deps, info, _) = valid_initialization(
        AndrAddr::from_string(MOCK_SCHEMA_ADO),
        None,
        form_config,
        None,
        instantiation_timestamp,
    );
    let err = open_form(deps.as_mut(), info.sender.as_ref(), execute_timestamp).unwrap_err();
    assert_eq!(expected_err, err);
}

#[test_case(
    FormConfig {
        start_time: Some(Expiry::AtTime(Milliseconds::from_nanos(2000000000000_u64))),
        end_time: Some(Expiry::AtTime(Milliseconds::from_nanos(3000000000000_u64))),
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    500000000000_u64,
    1000000000000_u64;
    "Valid timestamp at execution with saved start and end time_1"
)]
#[test_case(
    FormConfig {
        start_time: Some(Expiry::AtTime(Milliseconds::from_nanos(2000000000000_u64))),
        end_time: Some(Expiry::AtTime(Milliseconds::from_nanos(3000000000000_u64))),
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    500000000000_u64,
    4000000000000_u64;
    "Valid timestamp at execution with saved start and end time_2"
)]
#[test_case(
    FormConfig {
        start_time: None,
        end_time: None,
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    500000000000_u64,
    1000000000000_u64;
    "Valid timestamp at execution with none start and end time"
)]
#[test_case(
    FormConfig {
        start_time: None,
        end_time: Some(Expiry::AtTime(Milliseconds::from_nanos(4000000000000_u64))),
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    500000000000_u64,
    1000000000000_u64;
    "Valid timestamp at execution with end time_1"
)]
#[test_case(
    FormConfig {
        start_time: None,
        end_time: Some(Expiry::AtTime(Milliseconds::from_nanos(1000000000000_u64))),
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    500000000000_u64,
    3000000000000_u64;
    "Valid timestamp at execution with end time_2"
)]
fn test_success_open_form(
    form_config: FormConfig,
    instantiation_timestamp: u64,
    execute_timestamp: u64,
) {
    let (mut deps, info, _) = valid_initialization(
        AndrAddr::from_string(MOCK_SCHEMA_ADO),
        None,
        form_config.clone(),
        None,
        instantiation_timestamp,
    );
    let res = open_form(deps.as_mut(), info.sender.as_ref(), execute_timestamp);
    assert!(res.is_ok());

    let start_time = START_TIME.load(&deps.storage).unwrap();
    let expected_saved_start_time = if START_TIME.load(&deps.storage).unwrap().is_some() {
        Some(Milliseconds::from_nanos(execute_timestamp).plus_milliseconds(Milliseconds(1)))
    } else {
        None
    };
    assert_eq!(expected_saved_start_time, start_time);

    let end_time = END_TIME.load(&deps.storage).unwrap();
    // println!(
    //     "//====================     endtime: {:?}     ====================//",
    //     end_time
    // );

    let saved_start_time = if let Some(start_time) = form_config.start_time {
        let mut env = mock_env();
        env.block.time = Timestamp::from_nanos(instantiation_timestamp);
        Some(start_time.get_time(&env.block))
    } else {
        None
    };
    let saved_end_time = if let Some(end_time) = form_config.end_time {
        let mut env = mock_env();
        env.block.time = Timestamp::from_nanos(instantiation_timestamp);
        Some(end_time.get_time(&env.block))
    } else {
        None
    };
    let execute_time = Milliseconds::from_nanos(execute_timestamp);
    match saved_start_time {
        Some(saved_start_time) => match saved_end_time {
            Some(saved_end_time) => {
                if saved_start_time.gt(&execute_time) {
                    assert_eq!(end_time, Some(saved_end_time))
                } else if saved_end_time.gt(&execute_time) {
                    assert_eq!(end_time, None);
                }
            }
            None => {
                if saved_start_time.gt(&execute_time) {
                    assert_eq!(end_time, None);
                }
            }
        },
        None => {
            if let Some(saved_end_time) = saved_end_time {
                if execute_time.gt(&saved_end_time) {
                    assert_eq!(end_time, None);
                } else {
                    assert_eq!(end_time, Some(saved_end_time))
                }
            }
        }
    }
}

#[test_case(
    FormConfig {
        start_time: Some(Expiry::AtTime(Milliseconds::from_nanos(2000000000000_u64))),
        end_time: Some(Expiry::AtTime(Milliseconds::from_nanos(3000000000000_u64))),
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    500000000000_u64,
    1000000000000_u64,
    ContractError::CustomError {
        msg: format!("Not opened yet. Will be opend at {:?}", Milliseconds::from_nanos(2000000000000_u64)),
    };
    "Invalid timestamp at execution with saved start and end time-1"
)]
#[test_case(
    FormConfig {
        start_time: Some(Expiry::AtTime(Milliseconds::from_nanos(2000000000000_u64))),
        end_time: Some(Expiry::AtTime(Milliseconds::from_nanos(3000000000000_u64))),
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    500000000000_u64,
    4000000000000_u64,
    ContractError::CustomError {
        msg: format!("Already closed. Closed at {:?}", Milliseconds::from_nanos(3000000000000_u64)),
    };
    "Invalid timestamp at execution with saved start and end time-2"
)]
#[test_case(
    FormConfig {
        start_time: None,
        end_time: Some(Expiry::AtTime(Milliseconds::from_nanos(3000000000000_u64))),
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    500000000000_u64,
    4000000000000_u64,
    ContractError::CustomError {
        msg: "Not opened yet".to_string(),
    };
    "Invalid timestamp at execution with none start time"
)]
#[test_case(
    FormConfig {
        start_time: Some(Expiry::AtTime(Milliseconds::from_nanos(2000000000000_u64))),
        end_time: None,
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    500000000000_u64,
    1000000000000_u64,
    ContractError::CustomError {
        msg: format!("Not opened yet. Will be opend at {:?}", Milliseconds::from_nanos(2000000000000_u64)),
    };
    "Invalid timestamp at execution with start time"
)]
#[test_case(
    FormConfig {
        start_time: None,
        end_time: None,
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    },
    500000000000_u64,
    1000000000000_u64,
    ContractError::CustomError {
        msg: "Not opened yet".to_string(),
    };
    "Invalid timestamp at execution with none start and end time"
)]
fn test_failed_close_form(
    form_config: FormConfig,
    instantiation_timestamp: u64,
    execute_timestamp: u64,
    expected_err: ContractError,
) {
    let (mut deps, info, _) = valid_initialization(
        AndrAddr::from_string(MOCK_SCHEMA_ADO),
        None,
        form_config,
        None,
        instantiation_timestamp,
    );
    let err = close_form(deps.as_mut(), info.sender.as_ref(), execute_timestamp).unwrap_err();
    assert_eq!(expected_err, err);
}

#[test]
fn test_submit_form_allowed_multiple_submission() {
    let form_config = FormConfig {
        start_time: None,
        end_time: None,
        allow_multiple_submissions: true,
        allow_edit_submission: true,
    };
    let (mut deps, info, _) = valid_initialization(
        AndrAddr::from_string(MOCK_SCHEMA_ADO),
        None,
        form_config,
        None,
        5000000000_u64,
    );
    open_form(deps.as_mut(), info.sender.as_ref(), 10000000000_u64).unwrap();

    let form_status = query_form_status(deps.as_ref(), 20000000000_u64).unwrap();
    assert_eq!(form_status, GetFormStatusResponse::Opened);

    submit_form(
        deps.as_mut(),
        "user1",
        "valid_data1".to_string(),
        20000000000_u64,
    )
    .unwrap();
    submit_form(
        deps.as_mut(),
        "user1",
        "valid_data2".to_string(),
        30000000000_u64,
    )
    .unwrap();
    submit_form(
        deps.as_mut(),
        "user2",
        "valid_data3".to_string(),
        40000000000_u64,
    )
    .unwrap();
    submit_form(
        deps.as_mut(),
        "user3",
        "valid_data4".to_string(),
        50000000000_u64,
    )
    .unwrap();
    submit_form(
        deps.as_mut(),
        "user4",
        "valid_data5".to_string(),
        60000000000_u64,
    )
    .unwrap();
    submit_form(
        deps.as_mut(),
        "user4",
        "valid_data6".to_string(),
        70000000000_u64,
    )
    .unwrap();
    submit_form(
        deps.as_mut(),
        "user4",
        "valid_data7".to_string(),
        80000000000_u64,
    )
    .unwrap();
    let err = submit_form(
        deps.as_mut(),
        "user4",
        "invalid_data8".to_string(),
        85000000000_u64,
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::CustomError {
            msg: "Invalid data against schema".to_string(),
        }
    );

    let all_submissions = query_all_submissions(deps.as_ref())
        .unwrap()
        .all_submissions;
    assert_eq!(all_submissions.len(), 7_usize);

    delete_submission(
        deps.as_mut(),
        info.sender.as_ref(),
        4,
        AndrAddr::from_string("user3"),
    )
    .unwrap();
    let all_submissions = query_all_submissions(deps.as_ref())
        .unwrap()
        .all_submissions;
    assert_eq!(all_submissions.len(), 6_usize);

    close_form(deps.as_mut(), info.sender.as_ref(), 90000000000_u64).unwrap();

    let form_status = query_form_status(deps.as_ref(), 100000000000_u64).unwrap();
    assert_eq!(form_status, GetFormStatusResponse::Closed);

    let err = submit_form(
        deps.as_mut(),
        "user5",
        "valid_data8".to_string(),
        110000000000_u64,
    )
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::CustomError {
            msg: format!(
                "Already closed. Closed at {:?}",
                Milliseconds::from_nanos(90000000000_u64).plus_milliseconds(Milliseconds(1))
            )
        }
    );
}

#[test]
fn test_submit_form_disallowed_multiple_submission_disallowed_edit() {
    let form_config = FormConfig {
        start_time: None,
        end_time: None,
        allow_multiple_submissions: false,
        allow_edit_submission: false,
    };
    let (mut deps, info, _) = valid_initialization(
        AndrAddr::from_string(MOCK_SCHEMA_ADO),
        None,
        form_config,
        None,
        5000000000_u64,
    );
    open_form(deps.as_mut(), info.sender.as_ref(), 10000000000_u64).unwrap();
    submit_form(
        deps.as_mut(),
        "user1",
        "valid_data1".to_string(),
        20000000000_u64,
    )
    .unwrap();
    let res = submit_form(
        deps.as_mut(),
        "user1",
        "valid_data2".to_string(),
        30000000000_u64,
    )
    .unwrap_err();
    assert_eq!(
        res,
        ContractError::CustomError {
            msg: "Multiple submissions are not allowed".to_string(),
        }
    );

    let res = edit_submission(
        deps.as_mut(),
        "user1",
        1,
        AndrAddr::from_string("user1"),
        "valid_data2".to_string(),
    )
    .unwrap_err();
    assert_eq!(
        res,
        ContractError::CustomError {
            msg: "Edit submission is not allowed".to_string(),
        }
    );
}

#[test]
fn test_submit_form_disallowed_multiple_submission_allowed_edit() {
    let form_config = FormConfig {
        start_time: None,
        end_time: None,
        allow_multiple_submissions: false,
        allow_edit_submission: true,
    };
    let (mut deps, info, _) = valid_initialization(
        AndrAddr::from_string(MOCK_SCHEMA_ADO),
        None,
        form_config,
        None,
        5000000000_u64,
    );
    open_form(deps.as_mut(), info.sender.as_ref(), 10000000000_u64).unwrap();
    submit_form(
        deps.as_mut(),
        "user1",
        "valid_data1".to_string(),
        20000000000_u64,
    )
    .unwrap();

    let res = edit_submission(
        deps.as_mut(),
        "user2",
        1,
        AndrAddr::from_string("user1"),
        "valid_data2".to_string(),
    )
    .unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    edit_submission(
        deps.as_mut(),
        "user1",
        1,
        AndrAddr::from_string("user1"),
        "valid_data2".to_string(),
    )
    .unwrap();
    let submission = query_submission(deps.as_ref(), 1, AndrAddr::from_string("user1"))
        .unwrap()
        .submission;
    assert_eq!(
        submission,
        Some(SubmissionInfo {
            submission_id: 1,
            wallet_address: Addr::unchecked("user1"),
            data: "valid_data2".to_string()
        })
    );

    let schema = query_schema(deps.as_ref()).unwrap();
    assert_eq!(
        schema,
        GetSchemaResponse {
            schema: "{\"$schema\":\"http://json-schema.org/draft-07/schema#\",\"additionalProperties\":false,\"properties\":{\"kernel_address\":{\"type\":\"string\"},\"owner\":{\"type\":[\"string\",\"null\"]},\"schema_json_string\":{\"type\":\"string\"}},\"required\":[\"kernel_address\",\"schema_json_string\"],\"title\":\"InstantiateMsg\",\"type\":\"object\"}".to_string(),
        }
    );
}
