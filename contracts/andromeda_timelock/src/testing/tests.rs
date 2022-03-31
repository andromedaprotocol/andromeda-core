use cosmwasm_std::{
    coins,
    testing::{mock_env, mock_info},
    Response, StdError,
};

use crate::contract::{execute, instantiate};
use andromeda_protocol::{
    testing::mock_querier::{mock_dependencies_custom, MOCK_ADDRESSLIST_CONTRACT},
    timelock::{ExecuteMsg, InstantiateMsg},
};
use common::{ado_base::modules::Module, error::ContractError, mission::AndrAddress};

#[test]
fn test_modules() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let owner = "owner";
    let info = mock_info(owner, &[]);
    let msg = InstantiateMsg {
        modules: Some(vec![Module {
            module_type: "address_list".to_string(),
            is_mutable: false,
            address: AndrAddress {
                identifier: MOCK_ADDRESSLIST_CONTRACT.to_owned(),
            },
        }]),
    };
    let res = instantiate(deps.as_mut(), env, info, msg.clone()).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "register_module")
            .add_attribute("module_idx", "1")
            .add_attribute("method", "instantiate")
            .add_attribute("type", "timelock"),
        res
    );

    let msg = ExecuteMsg::HoldFunds {
        condition: None,
        recipient: None,
    };
    let info = mock_info("anyone", &coins(100, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    assert_eq!(
        ContractError::Std(StdError::generic_err(
            "Querier contract error: InvalidAddress"
        )),
        res.unwrap_err()
    );

    let info = mock_info("sender", &coins(100, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
    assert_eq!(
        Response::new()
            .add_attribute("action", "hold_funds")
            .add_attribute("sender", "sender")
            .add_attribute("recipient", "Addr(\"sender\")")
            .add_attribute("condition", "None"),
        res
    );
}
