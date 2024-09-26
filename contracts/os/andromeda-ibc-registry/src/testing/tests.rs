use andromeda_std::amp::AndrAddr;
use andromeda_std::os::ibc_registry::InstantiateMsg;
use andromeda_std::testing::mock_querier::MOCK_KERNEL_CONTRACT;
use cosmwasm_std::testing::mock_env;
use cosmwasm_std::testing::{mock_dependencies, mock_info};
use cosmwasm_std::Addr;

use crate::contract::instantiate;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        owner: None,
        kernel_address: Addr::unchecked(MOCK_KERNEL_CONTRACT),
        service_address: AndrAddr::from_string("service_address"),
    };
    let env = mock_env();

    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}
// The rest of the testing can be found in ibc registry's integration test
