use super::mock::{proper_initialization, query_split_result};
use andromeda_modules::string_utils::Delimiter;

#[test]
fn test_instantiation() {
    proper_initialization();
}

#[test]
fn test_query_split_whitespace() {
    let (deps, _) = proper_initialization();

    let input = "This is test case of string split".to_string();
    let delimiter = Delimiter::WhiteSpace;

    let query_res = query_split_result(deps.as_ref(), input, delimiter)
        .unwrap()
        .split_result;
    assert_eq!(
        query_res,
        vec![
            "This".to_string(),
            "is".to_string(),
            "test".to_string(),
            "case".to_string(),
            "of".to_string(),
            "string".to_string(),
            "split".to_string(),
        ]
    );
}
