use super::mock::{proper_initialization, query_block_height};
use cosmwasm_std::testing::mock_env;

#[test]
fn test_instantiation() {
    proper_initialization();
}

#[test]
fn test_query_split_whitespace() {
    let (deps, _) = proper_initialization();

    let query_res = query_block_height(deps.as_ref()).unwrap().block_height;
    let expected_res = mock_env().block.height;
    assert_eq!(query_res, expected_res);
}
