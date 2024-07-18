use andromeda_modules::string_utils::Delimiter;
use cosmwasm_std::Attribute;
use super::mock::{
    proper_initialization, try_split, query_split_result,
};

#[test]
fn test_instantiation() {
    proper_initialization();
}

#[test]
fn test_query_split_result_when_no_result() {
    let (deps, _) = proper_initialization();
    let res = query_split_result(deps.as_ref()).unwrap().split_result;
    assert_eq!(res, vec!["No split result found.".to_string()]);
}

#[test]
fn test_execute_split_other() {
    let (mut deps, info) = proper_initialization();

    let input = "1,2,3,4,5,6".to_string();
    let delimiter = Delimiter::Other { limiter: ",".to_string() };
    let res = try_split(deps.as_mut(), input, delimiter, info.sender.as_ref()).unwrap();
    assert_eq!(res.attributes, vec![
        Attribute { key: "action".to_string(), value: "StringUtilsSplit".to_string() },
        Attribute { key: "sender".to_string(), value: "creator".to_string() },
    ]);

    let query_res = query_split_result(deps.as_ref()).unwrap().split_result;
    assert_eq!(query_res, vec![
        "1".to_string(),
        "2".to_string(),
        "3".to_string(),
        "4".to_string(),
        "5".to_string(),
        "6".to_string(),
    ]);
}

#[test]
fn test_execute_split_whitespace() {
    let (mut deps, info) = proper_initialization();

    let input = "This is test case of string split".to_string();
    let delimiter = Delimiter::WhiteSpace;
    let res = try_split(deps.as_mut(), input, delimiter, info.sender.as_ref()).unwrap();
    assert_eq!(res.attributes, vec![
        Attribute { key: "action".to_string(), value: "StringUtilsSplit".to_string() },
        Attribute { key: "sender".to_string(), value: "creator".to_string() },
    ]);

    let query_res = query_split_result(deps.as_ref()).unwrap().split_result;
    assert_eq!(query_res, vec![
        "This".to_string(),
        "is".to_string(),
        "test".to_string(),
        "case".to_string(),
        "of".to_string(),
        "string".to_string(),
        "split".to_string(),
    ]);
}
