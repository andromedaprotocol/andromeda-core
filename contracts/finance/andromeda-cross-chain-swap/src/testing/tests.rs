use andromeda_std::ado_base::modules::Module;

use cosmwasm_std::{
    testing::{mock_env, mock_info},
    DepsMut, Response,
};

pub const OWNER: &str = "creator";

use super::mock_querier::MOCK_KERNEL_CONTRACT;

use crate::{contract::instantiate, testing::mock_querier::mock_dependencies_custom};
use andromeda_finance::cross_chain_swap::InstantiateMsg;

fn init(deps: DepsMut, _modules: Option<Vec<Module>>) -> Response {
    let msg = InstantiateMsg {
        owner: Some(OWNER.to_owned()),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
    };

    let info = mock_info("owner", &[]);
    instantiate(deps, mock_env(), info, msg).unwrap()
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);
    let res = init(deps.as_mut(), None);
    assert_eq!(0, res.messages.len());
}
