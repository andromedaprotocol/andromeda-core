use crate::contract::instantiate;

use amp::kernel::InstantiateMsg;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {};
    let env = mock_env();

    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}
