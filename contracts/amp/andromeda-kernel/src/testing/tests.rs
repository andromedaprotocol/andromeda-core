use crate::contract::{execute, instantiate, query};

use crate::state::CODE_ID;
use ado_base::ADOContract;
use andromeda_app::adodb::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_testing::testing::mock_querier::mock_dependencies_custom;
use common::{ado_base::AndromedaQuery, error::ContractError};
use cosmwasm_std::{
    attr, from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Response,
};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {};
    let env = mock_env();

    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}
