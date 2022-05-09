use cosmwasm_std::{
    coins,
    testing::{mock_env, mock_info},
    Response, StdError,
};

use crate::contract::{execute, instantiate};
use andromeda_finance::timelock::{ExecuteMsg, InstantiateMsg};
use andromeda_testing::testing::mock_querier::{
    mock_dependencies_custom, MOCK_ADDRESSLIST_CONTRACT,
};
use common::{
    ado_base::{modules::Module, AndromedaMsg},
    error::ContractError,
    mission::AndrAddress,
};

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
    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();

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
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        Response::new()
            .add_attribute("action", "hold_funds")
            .add_attribute("sender", "sender")
            .add_attribute("recipient", "Addr(\"sender\")")
            .add_attribute("condition", "None"),
        res
    );
}

#[test]
fn test_update_mission_contract() {
    let mut deps = mock_dependencies_custom(&[]);

    let modules: Vec<Module> = vec![Module {
        module_type: "address_list".to_string(),
        address: AndrAddress {
            identifier: MOCK_ADDRESSLIST_CONTRACT.to_owned(),
        },
        is_mutable: false,
    }];

    let info = mock_info("mission_contract", &[]);
    let msg = InstantiateMsg {
        modules: Some(modules),
    };
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::UpdateMissionContract {
        address: "mission_contract".to_string(),
    });

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "update_mission_contract")
            .add_attribute("address", "mission_contract"),
        res
    );
}
