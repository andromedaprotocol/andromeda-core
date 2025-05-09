use super::mock::{proper_initialization, query_date_time};
use andromeda_math::date_time::{GetDateTimeResponse, Timezone};

#[test]
fn test_instantiation() {
    proper_initialization();
}

#[test]
fn test_query_date_time() {
    let (deps, _) = proper_initialization();

    // UTC+3
    let query_res = query_date_time(deps.as_ref(), Some(Timezone::UtcPlus3)).unwrap();
    assert_eq!(
        query_res,
        GetDateTimeResponse {
            day_of_week: "Wed".to_string(),
            date_time: "2019-10-23 05-23-39".to_string(),
        }
    );

    // UTC-9
    let query_res = query_date_time(deps.as_ref(), Some(Timezone::UtcMinus9)).unwrap();
    assert_eq!(
        query_res,
        GetDateTimeResponse {
            day_of_week: "Tue".to_string(),
            date_time: "2019-10-22 17-23-39".to_string(),
        }
    );

    // UTC-2:30
    let query_res = query_date_time(deps.as_ref(), Some(Timezone::UtcMinus2_30)).unwrap();
    assert_eq!(
        query_res,
        GetDateTimeResponse {
            day_of_week: "Tue".to_string(),
            date_time: "2019-10-22 23-53-39".to_string(),
        }
    );

    // UTC
    let query_res = query_date_time(deps.as_ref(), None).unwrap();
    assert_eq!(
        query_res,
        GetDateTimeResponse {
            day_of_week: "Wed".to_string(),
            date_time: "2019-10-23 02-23-39".to_string(),
        }
    );

    // UTC+10:30
    let query_res = query_date_time(deps.as_ref(), Some(Timezone::UtcPlus10_30)).unwrap();
    assert_eq!(
        query_res,
        GetDateTimeResponse {
            day_of_week: "Wed".to_string(),
            date_time: "2019-10-23 12-53-39".to_string(),
        }
    );

    // UTC+12:45
    let query_res = query_date_time(deps.as_ref(), Some(Timezone::UtcPlus12_45)).unwrap();
    assert_eq!(
        query_res,
        GetDateTimeResponse {
            day_of_week: "Wed".to_string(),
            date_time: "2019-10-23 15-08-39".to_string(),
        }
    );

    // UTC+14
    let query_res = query_date_time(deps.as_ref(), Some(Timezone::UtcPlus14)).unwrap();
    assert_eq!(
        query_res,
        GetDateTimeResponse {
            day_of_week: "Wed".to_string(),
            date_time: "2019-10-23 16-23-39".to_string(),
        }
    );
}
