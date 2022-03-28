use crate::{
    contract::*,
    state::{ADO_ADDRESSES, ADO_DESCRIPTORS},
};
use andromeda_protocol::{
    mission::{ExecuteMsg, InstantiateMsg, MissionComponent},
    testing::mock_querier::mock_dependencies_custom,
};
use common::{ado_base::AndromedaMsg, error::ContractError};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, ContractResult, CosmosMsg, Event, Reply, StdError, SubMsgExecutionResponse,
    WasmMsg,
};

#[test]
fn test_empty_instantiation() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        operators: vec![],
        mission: vec![],
        xfer_ado_ownership: false,
        name: String::from("Some Mission"),
        primitive_contract: String::from("primitive_contract"),
    };
    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_instantiation() {
    let mut deps = mock_dependencies_custom(&[]);

    let msg = InstantiateMsg {
        operators: vec![],
        mission: vec![MissionComponent {
            name: "token".to_string(),
            ado_type: "cw721".to_string(),
            instantiate_msg: to_binary(&true).unwrap(),
        }],
        xfer_ado_ownership: false,
        name: String::from("Some Mission"),
        primitive_contract: String::from("primitive_contract"),
    };
    let info = mock_info("creator", &[]);

    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(1, res.messages.len());
    match res.messages[0].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Instantiate { code_id, msg, .. }) => {
            assert_eq!(4, code_id);
            assert_eq!(to_binary(&true).unwrap(), msg)
        }
        _ => panic!("Invalid msg type"),
    }
}

// TODO: Figure out how to test this
// #[test]
// fn test_instantiation_xfer_ownership() {
//     let mut deps = mock_dependencies_custom(&[]);

//     let msg = InstantiateMsg {
//         operators: vec![],
//         mission: vec![MissionComponent {
//             name: "token".to_string(),
//             ado_type: "cw721".to_string(),
//             instantiate_msg: to_binary(&true).unwrap(),
//         }],
//         xfer_ado_ownership: true,
//         name: String::from("Some Mission"),
//         primitive_contract: String::from("primitive_contract"),
//     };
//     let info = mock_info("creator", &[]);

//     ADO_ADDRESSES
//         .save(
//             deps.as_mut().storage,
//             "token",
//             &Addr::unchecked("tokenaddress"),
//         )
//         .unwrap();

//     let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
//     assert_eq!(2, res.messages.len());

//     match res.messages[0].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Instantiate { code_id, msg, .. }) => {
//             assert_eq!(4, code_id);
//             assert_eq!(to_binary(&true).unwrap(), msg)
//         }
//         _ => panic!("Invalid msg type"),
//     }

//     match res.messages[1].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             contract_addr, msg, ..
//         }) => {
//             assert_eq!("tokenaddress".to_string(), contract_addr);
//             assert_eq!(
//                 to_binary(&AndromedaMsg::UpdateOwner {
//                     address: "creator".to_string()
//                 })
//                 .unwrap(),
//                 msg
//             );
//         }
//         _ => panic!("Invalid msg type"),
//     }
// }

#[test]
fn test_add_mission_component_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        operators: vec![],
        mission: vec![],
        xfer_ado_ownership: false,
        name: String::from("Some Mission"),
        primitive_contract: String::from("primitive_contract"),
    };

    instantiate(deps.as_mut(), env.clone(), info, inst_msg).unwrap();

    let unauth_info = mock_info("anyone", &[]);
    let msg = ExecuteMsg::AddMissionComponent {
        component: MissionComponent {
            name: "token".to_string(),
            ado_type: "cw721".to_string(),
            instantiate_msg: to_binary(&true).unwrap(),
        },
    };

    let err = execute(deps.as_mut(), env, unauth_info, msg).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err);
}

#[test]
fn test_add_mission_component_duplicate_name() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        operators: vec![],
        mission: vec![MissionComponent {
            name: "token".to_string(),
            ado_type: "cw721".to_string(),
            instantiate_msg: to_binary(&true).unwrap(),
        }],
        xfer_ado_ownership: false,
        name: String::from("Some Mission"),
        primitive_contract: String::from("primitive_contract"),
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();
    ADO_ADDRESSES
        .save(
            deps.as_mut().storage,
            "token",
            &Addr::unchecked("someaddress"),
        )
        .unwrap();

    let msg = ExecuteMsg::AddMissionComponent {
        component: MissionComponent {
            name: "token".to_string(),
            ado_type: "cw721".to_string(),
            instantiate_msg: to_binary(&true).unwrap(),
        },
    };

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(ContractError::NameAlreadyTaken {}, err);
}

#[test]
fn test_add_mission_component() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        operators: vec![],
        mission: vec![],
        xfer_ado_ownership: false,
        name: String::from("Some Mission"),
        primitive_contract: String::from("primitive_contract"),
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();

    let msg = ExecuteMsg::AddMissionComponent {
        component: MissionComponent {
            name: "token".to_string(),
            ado_type: "cw721".to_string(),
            instantiate_msg: to_binary(&true).unwrap(),
        },
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    match res.messages[0].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Instantiate { code_id, msg, .. }) => {
            assert_eq!(4, code_id);
            assert_eq!(to_binary(&true).unwrap(), msg)
        }
        _ => panic!("Invalid msg type"),
    }
}

#[test]
fn test_claim_ownership_unauth() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        operators: vec![],
        mission: vec![],
        xfer_ado_ownership: false,
        name: String::from("Some Mission"),
        primitive_contract: String::from("primitive_contract"),
    };

    instantiate(deps.as_mut(), env.clone(), info, inst_msg).unwrap();

    let unauth_info = mock_info("anyone", &[]);
    let msg = ExecuteMsg::ClaimOwnership { name: None };

    let err = execute(deps.as_mut(), env, unauth_info, msg).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err);
}

#[test]
fn test_claim_ownership_not_found() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        operators: vec![],
        mission: vec![],
        xfer_ado_ownership: false,
        name: String::from("Some Mission"),
        primitive_contract: String::from("primitive_contract"),
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();

    let msg = ExecuteMsg::ClaimOwnership {
        name: Some("token".to_string()),
    };

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        ContractError::Std(StdError::NotFound {
            kind: "cosmwasm_std::addresses::Addr".to_string()
        }),
        err
    );
}

#[test]
fn test_claim_ownership_empty() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        operators: vec![],
        mission: vec![],
        xfer_ado_ownership: false,
        name: String::from("Some Mission"),
        primitive_contract: String::from("primitive_contract"),
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();

    let msg = ExecuteMsg::ClaimOwnership { name: None };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_claim_ownership_all() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        operators: vec![],
        mission: vec![],
        xfer_ado_ownership: false,
        name: String::from("Some Mission"),
        primitive_contract: String::from("primitive_contract"),
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();
    ADO_ADDRESSES
        .save(
            deps.as_mut().storage,
            "token",
            &Addr::unchecked("tokenaddress".to_string()),
        )
        .unwrap();
    ADO_ADDRESSES
        .save(
            deps.as_mut().storage,
            "anchor",
            &Addr::unchecked("anchoraddress".to_string()),
        )
        .unwrap();

    let msg = ExecuteMsg::ClaimOwnership { name: None };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(2, res.messages.len());

    match res.messages[0].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr, msg, ..
        }) => {
            assert_eq!("anchoraddress".to_string(), contract_addr);
            assert_eq!(
                to_binary(&AndromedaMsg::UpdateOwner {
                    address: "creator".to_string()
                })
                .unwrap(),
                msg
            );
        }
        _ => panic!("Invalid msg type"),
    }
    match res.messages[1].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr, msg, ..
        }) => {
            assert_eq!("tokenaddress".to_string(), contract_addr);
            assert_eq!(
                to_binary(&AndromedaMsg::UpdateOwner {
                    address: "creator".to_string()
                })
                .unwrap(),
                msg
            );
        }
        _ => panic!("Invalid msg type"),
    }
}

#[test]
fn test_claim_ownership() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        operators: vec![],
        mission: vec![],
        xfer_ado_ownership: false,
        name: String::from("Some Mission"),
        primitive_contract: String::from("primitive_contract"),
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();
    ADO_ADDRESSES
        .save(
            deps.as_mut().storage,
            "token",
            &Addr::unchecked("tokenaddress".to_string()),
        )
        .unwrap();
    ADO_ADDRESSES
        .save(
            deps.as_mut().storage,
            "anchor",
            &Addr::unchecked("anchoraddress".to_string()),
        )
        .unwrap();

    let msg = ExecuteMsg::ClaimOwnership {
        name: Some("token".to_string()),
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(1, res.messages.len());

    match res.messages[0].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr, msg, ..
        }) => {
            assert_eq!("tokenaddress".to_string(), contract_addr);
            assert_eq!(
                to_binary(&AndromedaMsg::UpdateOwner {
                    address: "creator".to_string()
                })
                .unwrap(),
                msg
            );
        }
        _ => panic!("Invalid msg type"),
    }
}

#[test]
fn test_proxy_message_unauth() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        operators: vec![],
        mission: vec![],
        xfer_ado_ownership: false,
        name: String::from("Some Mission"),
        primitive_contract: String::from("primitive_contract"),
    };

    instantiate(deps.as_mut(), env.clone(), info, inst_msg).unwrap();

    let unauth_info = mock_info("anyone", &[]);
    let msg = ExecuteMsg::ProxyMessage {
        name: "token".to_string(),
        msg: to_binary(&true).unwrap(),
    };

    let err = execute(deps.as_mut(), env, unauth_info, msg).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err);
}

#[test]
fn test_proxy_message_not_found() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        operators: vec![],
        mission: vec![],
        xfer_ado_ownership: false,
        name: String::from("Some Mission"),
        primitive_contract: String::from("primitive_contract"),
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();

    let msg = ExecuteMsg::ProxyMessage {
        name: "token".to_string(),
        msg: to_binary(&true).unwrap(),
    };

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        ContractError::Std(StdError::NotFound {
            kind: "cosmwasm_std::addresses::Addr".to_string()
        }),
        err
    );
}

#[test]
fn test_proxy_message() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        operators: vec![],
        mission: vec![],
        xfer_ado_ownership: false,
        name: String::from("Some Mission"),
        primitive_contract: String::from("primitive_contract"),
    };
    ADO_ADDRESSES
        .save(
            deps.as_mut().storage,
            "token",
            &Addr::unchecked("tokenaddress".to_string()),
        )
        .unwrap();
    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();

    let msg = ExecuteMsg::ProxyMessage {
        name: "token".to_string(),
        msg: to_binary(&true).unwrap(),
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    match res.messages[0].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr, msg, ..
        }) => {
            assert_eq!("tokenaddress".to_string(), contract_addr);
            assert_eq!(to_binary(&true).unwrap(), msg);
        }
        _ => panic!("Invalid msg type"),
    }
}

#[test]
fn test_update_address_unauth() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        operators: vec![],
        mission: vec![],
        xfer_ado_ownership: false,
        name: String::from("Some Mission"),
        primitive_contract: String::from("primitive_contract"),
    };

    ADO_ADDRESSES
        .save(
            deps.as_mut().storage,
            "token",
            &Addr::unchecked("tokenaddress".to_string()),
        )
        .unwrap();
    instantiate(deps.as_mut(), env.clone(), info, inst_msg).unwrap();

    let unauth_info = mock_info("anyone", &[]);
    let msg = ExecuteMsg::UpdateAddress {
        name: "token".to_string(),
        addr: "newtokenaddress".to_string(),
    };

    let err = execute(deps.as_mut(), env, unauth_info, msg).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err);
}

#[test]
fn test_update_address_not_found() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        operators: vec![],
        mission: vec![],
        xfer_ado_ownership: false,
        name: String::from("Some Mission"),
        primitive_contract: String::from("primitive_contract"),
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();

    let msg = ExecuteMsg::UpdateAddress {
        name: "token".to_string(),
        addr: "newtokenaddress".to_string(),
    };

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        ContractError::Std(StdError::NotFound {
            kind: "cosmwasm_std::addresses::Addr".to_string()
        }),
        err
    );
}

#[test]
fn test_update_address() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        operators: vec![],
        mission: vec![],
        xfer_ado_ownership: false,
        name: String::from("Some Mission"),
        primitive_contract: String::from("primitive_contract"),
    };

    ADO_ADDRESSES
        .save(
            deps.as_mut().storage,
            "token",
            &Addr::unchecked("tokenaddress".to_string()),
        )
        .unwrap();
    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();

    let msg = ExecuteMsg::UpdateAddress {
        name: "token".to_string(),
        addr: "newtokenaddress".to_string(),
    };

    execute(deps.as_mut(), env, info, msg).unwrap();

    let addr = ADO_ADDRESSES.load(deps.as_ref().storage, "token").unwrap();
    assert_eq!(Addr::unchecked("newtokenaddress"), addr)
}

#[test]
fn test_reply_assign_mission() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let mock_mission_component = MissionComponent {
        ado_type: "cw721".to_string(),
        name: "token".to_string(),
        instantiate_msg: to_binary(&true).unwrap(),
    };
    let component_idx = 1;
    ADO_DESCRIPTORS
        .save(
            deps.as_mut().storage,
            &component_idx.to_string(),
            &mock_mission_component,
        )
        .unwrap();

    let mock_reply_event = Event::new("instantiate")
        .add_attribute("contract_address".to_string(), "tokenaddress".to_string());

    let mock_reply = Reply {
        id: component_idx,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            data: None,
            events: vec![mock_reply_event],
        }),
    };

    let res = reply(deps.as_mut(), env.clone(), mock_reply).unwrap();
    assert_eq!(1, res.messages.len());

    match res.messages[0].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr, msg, ..
        }) => {
            assert_eq!("tokenaddress".to_string(), contract_addr);
            assert_eq!(
                to_binary(&AndromedaMsg::UpdateMissionContract {
                    address: env.contract.address.to_string()
                })
                .unwrap(),
                msg
            );
        }
        _ => panic!("Invalid msg type"),
    }
}
