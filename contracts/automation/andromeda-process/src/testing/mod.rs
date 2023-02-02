use crate::{
    contract::*,
    state::{ADO_ADDRESSES, ADO_DESCRIPTORS, FIRST_ADOS},
};
// use andromeda_automation::condition::ExecuteMsg as ConditionExecuteMsg;
use andromeda_automation::condition::ExecuteMsg as ConditionExecuteMsg;
use andromeda_automation::process::{ExecuteMsg, InstantiateMsg, ProcessComponent};
use andromeda_testing::{
    reply::MsgInstantiateContractResponse, testing::mock_querier::mock_dependencies_custom,
};
use common::{ado_base::AndromedaMsg, encode_binary, error::ContractError};
use cosmwasm_std::{
    attr,
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, CosmosMsg, Empty, Event, Reply, ReplyOn, Response, StdError, SubMsg,
    SubMsgResponse, SubMsgResult, WasmMsg,
};
use prost::Message;
#[test]
fn test_empty_instantiation() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
        process: vec![],
        name: String::from("Some Process"),

        first_ados: vec!["condition_ado".to_string()],
        kernel_address: Some("kernel_contract".to_string()),
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
        process: vec![ProcessComponent {
            name: "token".to_string(),
            ado_type: "cw721".to_string(),
            instantiate_msg: to_binary(&true).unwrap(),
        }],
        name: String::from("Some Process"),

        first_ados: vec!["condition_ado".to_string()],
        kernel_address: Some("kernel_contract".to_string()),
    };
    let info = mock_info("creator", &[]);

    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(1, res.messages.len());
    let inst_submsg: SubMsg<Empty> = SubMsg {
        id: 1,
        msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
            code_id: 4,
            msg: to_binary(&true).unwrap(),
            funds: vec![],
            label: "Instantiate: cw721".to_string(),
            admin: Some("creator".to_string()),
        }),
        reply_on: ReplyOn::Always,
        gas_limit: None,
    };
    let expected = Response::new()
        .add_submessage(inst_submsg)
        .add_attributes(vec![
            attr("method", "instantiate"),
            attr("type", "process"),
            attr("owner", "creator"),
            attr("andr_process", "Some Process"),
        ]);

    assert_eq!(expected, res);

    assert_eq!(
        Addr::unchecked(""),
        ADO_ADDRESSES.load(deps.as_ref().storage, "token").unwrap()
    );
}

#[test]
fn test_instantiation_duplicate_components() {
    let mut deps = mock_dependencies_custom(&[]);

    let msg = InstantiateMsg {
        process: vec![
            ProcessComponent {
                name: "component".to_string(),
                ado_type: "cw721".to_string(),
                instantiate_msg: to_binary(&true).unwrap(),
            },
            ProcessComponent {
                name: "component".to_string(),
                ado_type: "cw20".to_string(),
                instantiate_msg: to_binary(&true).unwrap(),
            },
        ],
        name: String::from("Some Process"),

        first_ados: vec!["condition_ado".to_string()],
        kernel_address: Some("kernel_contract".to_string()),
    };
    let info = mock_info("creator", &[]);

    let res = instantiate(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::NameAlreadyTaken {}, res.unwrap_err());
}

#[test]
fn test_add_process_component_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        process: vec![],
        name: String::from("Some Process"),

        first_ados: vec!["condition_ado".to_string()],
        kernel_address: Some("kernel_contract".to_string()),
    };

    instantiate(deps.as_mut(), env.clone(), info, inst_msg).unwrap();

    let unauth_info = mock_info("anyone", &[]);
    let msg = ExecuteMsg::AddProcessComponent {
        component: ProcessComponent {
            name: "token".to_string(),
            ado_type: "cw721".to_string(),
            instantiate_msg: to_binary(&true).unwrap(),
        },
    };

    let err = execute(deps.as_mut(), env, unauth_info, msg).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err);
}

#[test]
fn test_add_process_component_duplicate_name() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        process: vec![ProcessComponent {
            name: "token".to_string(),
            ado_type: "cw721".to_string(),
            instantiate_msg: to_binary(&true).unwrap(),
        }],
        name: String::from("Some Process"),

        first_ados: vec!["condition_ado".to_string()],
        kernel_address: Some("kernel_contract".to_string()),
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();
    ADO_ADDRESSES
        .save(
            deps.as_mut().storage,
            "token",
            &Addr::unchecked("someaddress"),
        )
        .unwrap();

    let msg = ExecuteMsg::AddProcessComponent {
        component: ProcessComponent {
            name: "token".to_string(),
            ado_type: "cw721".to_string(),
            instantiate_msg: to_binary(&true).unwrap(),
        },
    };

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(ContractError::NameAlreadyTaken {}, err);
}

#[test]
fn test_add_process_component() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        process: vec![],
        name: String::from("Some Process"),

        first_ados: vec!["condition_ado".to_string()],
        kernel_address: Some("kernel_contract".to_string()),
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();

    let msg = ExecuteMsg::AddProcessComponent {
        component: ProcessComponent {
            name: "token".to_string(),
            ado_type: "cw721".to_string(),
            instantiate_msg: to_binary(&true).unwrap(),
        },
    };

    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

    assert_eq!(1, res.messages.len());
    let inst_submsg: SubMsg<Empty> = SubMsg {
        id: 1,
        msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
            code_id: 4,
            msg: to_binary(&true).unwrap(),
            funds: vec![],
            label: "Instantiate: cw721".to_string(),
            admin: Some("creator".to_string()),
        }),
        reply_on: ReplyOn::Always,
        gas_limit: None,
    };
    let expected = Response::new()
        .add_submessage(inst_submsg)
        .add_attributes(vec![
            attr("method", "add_process_component"),
            attr("name", "token"),
            attr("type", "cw721"),
        ]);

    assert_eq!(expected, res);

    assert_eq!(
        Addr::unchecked(""),
        ADO_ADDRESSES.load(deps.as_ref().storage, "token").unwrap()
    );

    let msg = ExecuteMsg::AddProcessComponent {
        component: ProcessComponent {
            name: "splitter".to_string(),
            ado_type: "splitter-ado".to_string(),
            instantiate_msg: to_binary(&true).unwrap(),
        },
    };

    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        FIRST_ADOS.load(&deps.storage).unwrap(),
        vec!["condition_ado".to_string()]
    )
}

#[test]
fn test_fire_condition_works() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        process: vec![],
        name: String::from("Some Process"),

        first_ados: vec!["condition1".to_string(), "condition2".to_string()],
        kernel_address: Some("kernel_contract".to_string()),
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();

    let msg = ExecuteMsg::AddProcessComponent {
        component: ProcessComponent {
            name: "condition1".to_string(),
            ado_type: "condition".to_string(),
            instantiate_msg: to_binary(&true).unwrap(),
        },
    };

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::AddProcessComponent {
        component: ProcessComponent {
            name: "condition2".to_string(),
            ado_type: "condition".to_string(),
            instantiate_msg: to_binary(&true).unwrap(),
        },
    };

    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

    assert_eq!(1, res.messages.len());
    let inst_submsg: SubMsg<Empty> = SubMsg {
        id: 2,
        msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
            code_id: 0,
            msg: to_binary(&true).unwrap(),
            funds: vec![],
            label: "Instantiate: condition".to_string(),
            admin: Some("creator".to_string()),
        }),
        reply_on: ReplyOn::Always,
        gas_limit: None,
    };
    let expected = Response::new()
        .add_submessage(inst_submsg)
        .add_attributes(vec![
            attr("method", "add_process_component"),
            attr("name", "condition2"),
            attr("type", "condition"),
        ]);

    assert_eq!(expected, res);

    assert_eq!(
        Addr::unchecked(""),
        ADO_ADDRESSES
            .load(deps.as_ref().storage, "condition1")
            .unwrap()
    );

    let msg = ExecuteMsg::AddProcessComponent {
        component: ProcessComponent {
            name: "splitter".to_string(),
            ado_type: "splitter-ado".to_string(),
            instantiate_msg: to_binary(&true).unwrap(),
        },
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    assert_eq!(
        FIRST_ADOS.load(&deps.storage).unwrap(),
        vec!["condition1".to_string(), "condition2".to_string()]
    );
    let msg = ExecuteMsg::Fire {};
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    println!("{res:?}");

    let expected_res = Response::new()
        .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "".to_string(),
            msg: encode_binary(&ConditionExecuteMsg::GetResults {}).unwrap(),
            funds: vec![],
        })))
        .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "".to_string(),
            msg: encode_binary(&ConditionExecuteMsg::GetResults {}).unwrap(),
            funds: vec![],
        })))
        .add_attribute("address", "".to_string())
        .add_attribute("address", "".to_string())
        .add_attribute("action", "fire_ado");
    assert_eq!(res, expected_res);
}

#[test]
fn test_claim_ownership_unauth() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        process: vec![],
        name: String::from("Some Process"),

        first_ados: vec!["condition_ado".to_string()],
        kernel_address: Some("kernel_contract".to_string()),
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
        process: vec![],
        name: String::from("Some Process"),

        first_ados: vec!["condition_ado".to_string()],
        kernel_address: Some("kernel_contract".to_string()),
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
        process: vec![],
        name: String::from("Some Process"),

        first_ados: vec!["condition_ado".to_string()],
        kernel_address: Some("kernel_contract".to_string()),
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
        process: vec![],
        name: String::from("Some Process"),

        first_ados: vec!["condition_ado".to_string()],
        kernel_address: Some("kernel_contract".to_string()),
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

    let exec_submsg: SubMsg<Empty> = SubMsg {
        id: 101,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "anchoraddress".to_string(),
            msg: to_binary(&ExecuteMsg::AndrReceive(AndromedaMsg::UpdateOwner {
                address: "creator".to_string(),
            }))
            .unwrap(),
            funds: vec![],
        }),
        reply_on: ReplyOn::Error,
        gas_limit: None,
    };
    let exec_submsg2: SubMsg<Empty> = SubMsg {
        id: 101,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "tokenaddress".to_string(),
            msg: to_binary(&ExecuteMsg::AndrReceive(AndromedaMsg::UpdateOwner {
                address: "creator".to_string(),
            }))
            .unwrap(),
            funds: vec![],
        }),
        reply_on: ReplyOn::Error,
        gas_limit: None,
    };
    let expected = Response::new()
        .add_submessages(vec![exec_submsg, exec_submsg2])
        .add_attributes(vec![attr("method", "claim_ownership")]);

    assert_eq!(expected, res)
}

#[test]
fn test_claim_ownership() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        process: vec![],
        name: String::from("Some Process"),

        first_ados: vec!["condition_ado".to_string()],
        kernel_address: Some("kernel_contract".to_string()),
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

    let exec_submsg: SubMsg<Empty> = SubMsg {
        id: 101,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "tokenaddress".to_string(),
            msg: to_binary(&ExecuteMsg::AndrReceive(AndromedaMsg::UpdateOwner {
                address: "creator".to_string(),
            }))
            .unwrap(),
            funds: vec![],
        }),
        reply_on: ReplyOn::Error,
        gas_limit: None,
    };
    let expected = Response::new()
        .add_submessage(exec_submsg)
        .add_attributes(vec![attr("method", "claim_ownership")]);

    assert_eq!(expected, res)
}

#[test]
fn test_proxy_message_unauth() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        process: vec![],
        name: String::from("Some Process"),

        first_ados: vec!["condition_ado".to_string()],
        kernel_address: Some("kernel_contract".to_string()),
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
        process: vec![],
        name: String::from("Some Process"),

        first_ados: vec!["condition_ado".to_string()],
        kernel_address: Some("kernel_contract".to_string()),
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
        process: vec![],
        name: String::from("Some Process"),

        first_ados: vec!["condition_ado".to_string()],
        kernel_address: Some("kernel_contract".to_string()),
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

    let exec_submsg: SubMsg<Empty> = SubMsg {
        id: 102,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "tokenaddress".to_string(),
            msg: to_binary(&true).unwrap(),
            funds: vec![],
        }),
        reply_on: ReplyOn::Error,
        gas_limit: None,
    };
    let expected = Response::new()
        .add_submessage(exec_submsg)
        .add_attributes(vec![
            attr("method", "process_message"),
            attr("recipient", "token"),
        ]);

    assert_eq!(expected, res)
}

#[test]
fn test_update_address_unauth() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let inst_msg = InstantiateMsg {
        process: vec![],
        name: String::from("Some Process"),

        first_ados: vec!["condition_ado".to_string()],
        kernel_address: Some("kernel_contract".to_string()),
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
        process: vec![],
        name: String::from("Some Process"),

        first_ados: vec!["condition_ado".to_string()],
        kernel_address: Some("kernel_contract".to_string()),
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
        process: vec![],
        name: String::from("Some Process"),

        first_ados: vec!["condition_ado".to_string()],
        kernel_address: Some("kernel_contract".to_string()),
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
fn test_reply_assign_process() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let mock_process_component = ProcessComponent {
        ado_type: "cw721".to_string(),
        name: "token".to_string(),
        instantiate_msg: to_binary(&true).unwrap(),
    };
    let component_idx = 1;
    ADO_DESCRIPTORS
        .save(
            deps.as_mut().storage,
            &component_idx.to_string(),
            &mock_process_component,
        )
        .unwrap();

    let mock_reply_event = Event::new("instantiate")
        .add_attribute("contract_address".to_string(), "tokenaddress".to_string());

    let instantiate_reply = MsgInstantiateContractResponse {
        contract_address: "tokenaddress".to_string(),
        data: vec![],
    };
    let mut encoded_instantiate_reply = Vec::<u8>::with_capacity(instantiate_reply.encoded_len());

    instantiate_reply
        .encode(&mut encoded_instantiate_reply)
        .unwrap();

    let mock_reply = Reply {
        id: component_idx,
        result: SubMsgResult::Ok(SubMsgResponse {
            data: Some(encoded_instantiate_reply.into()),
            events: vec![mock_reply_event],
        }),
    };

    let res = reply(deps.as_mut(), env.clone(), mock_reply).unwrap();
    assert_eq!(1, res.messages.len());

    let exec_submsg: SubMsg<Empty> = SubMsg {
        id: 103,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "tokenaddress".to_string(),
            msg: to_binary(&ExecuteMsg::AndrReceive(AndromedaMsg::UpdateAppContract {
                address: env.contract.address.to_string(),
            }))
            .unwrap(),
            funds: vec![],
        }),
        reply_on: ReplyOn::Error,
        gas_limit: None,
    };
    let expected = Response::new().add_submessage(exec_submsg);

    assert_eq!(expected, res);

    assert_eq!(
        Addr::unchecked("tokenaddress"),
        ADO_ADDRESSES.load(deps.as_ref().storage, "token").unwrap()
    );
}
