use super::{contract::*, state::ADO_ADDRESSES};
use crate::state::{ADO_DESCRIPTORS, ADO_IDX};
use andromeda_app::app::{AppComponent, ComponentType, ExecuteMsg, InstantiateMsg};
use andromeda_std::ado_base::ownership::OwnershipMessage;
use andromeda_std::testing::mock_querier::{
    mock_dependencies_custom, MOCK_ANCHOR_CONTRACT, MOCK_CW20_CONTRACT, MOCK_KERNEL_CONTRACT,
};
mod mock_querier;
use andromeda_std::{ado_base::AndromedaMsg, common::reply::ReplyId, error::ContractError};
use cosmwasm_std::{
    attr,
    testing::{message_info, mock_env},
    to_json_binary, Addr, Binary, CosmosMsg, Event, Reply, ReplyOn, Response, StdError, SubMsg,
    SubMsgResponse, SubMsgResult, WasmMsg,
};
use mock_querier::mock_dependencies_custom_v2;

#[test]
fn test_empty_instantiation() {
    let mut deps = mock_dependencies_custom(&[]);

    let msg = InstantiateMsg {
        app_components: vec![],
        name: String::from("Some App"),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        chain_info: None,
    };
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);

    // we can just call .unwrap() to assert this was a success
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(1, res.messages.len());
}

#[test]
#[ignore = "Requires updating mock implementation for CosmWasm 2.0+"]
fn test_instantiation() {
    let mut deps = mock_dependencies_custom_v2(&[]);
    let env = mock_env();
    let creator = deps.api.addr_make("app_creator");
    let info = message_info(&creator, &[]);
    let kernel_addr = deps.api.addr_make("kernel");

    // Init contract directly
    let msg = InstantiateMsg {
        app_components: vec![AppComponent {
            name: "main".to_string(),
            ado_type: "cw20".to_string(),
            component_type: ComponentType::New(to_json_binary(&true).unwrap()),
        }],
        name: String::from("Test App"),
        kernel_address: kernel_addr.to_string(),
        owner: None,
        chain_info: None,
    };

    let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Ensure that response has a number of SubMsg
    assert_eq!(res.messages.len(), 2);

    // Just check that messages have expected IDs and types, without detailed contents
    let first_msg = &res.messages[0];
    assert_eq!(first_msg.id, ReplyId::RegisterPath.repr());
    match &first_msg.msg {
        CosmosMsg::Wasm(WasmMsg::Execute { .. }) => {
            // VFS message is the correct type
        }
        _ => panic!("First message should be a WasmMsg::Execute"),
    }

    let second_msg = &res.messages[1];
    assert_eq!(second_msg.id, ReplyId::AssignApp.repr());
    match &second_msg.msg {
        CosmosMsg::Wasm(WasmMsg::Instantiate { .. }) => {
            // Component instantiation is the correct type
        }
        _ => panic!("Second message should be a WasmMsg::Instantiate"),
    }
}

#[test]
#[ignore = "Requires updating mock implementation for CosmWasm 2.0+"]
fn test_instantiation_duplicate_components() {
    let mut deps = mock_dependencies_custom(&[]);

    let msg = InstantiateMsg {
        app_components: vec![
            AppComponent {
                name: "component".to_string(),
                ado_type: "cw721".to_string(),
                component_type: ComponentType::New(to_json_binary(&true).unwrap()),
            },
            AppComponent {
                name: "component".to_string(),
                ado_type: "cw20".to_string(),
                component_type: ComponentType::New(to_json_binary(&true).unwrap()),
            },
        ],
        name: String::from("Some App"),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        chain_info: None,
    };
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);

    let res = instantiate(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::NameAlreadyTaken {}, res.unwrap_err());
}

#[test]
fn test_add_app_component_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);
    let inst_msg = InstantiateMsg {
        app_components: vec![],
        name: String::from("Some App"),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        chain_info: None,
    };

    instantiate(deps.as_mut(), env.clone(), info, inst_msg).unwrap();

    let anyone = deps.api.addr_make("anyone");
    let unauth_info = message_info(&anyone, &[]);
    let msg = ExecuteMsg::AddAppComponent {
        component: AppComponent {
            name: "token".to_string(),
            ado_type: "cw721".to_string(),
            component_type: ComponentType::New(to_json_binary(&true).unwrap()),
        },
    };

    let err = execute(deps.as_mut(), env, unauth_info, msg).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err);
}

#[test]
#[ignore = "Requires updating mock implementation for CosmWasm 2.0+"]
fn test_add_app_component_duplicate_name() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);
    let inst_msg = InstantiateMsg {
        app_components: vec![AppComponent {
            name: "token".to_string(),
            ado_type: "cw721".to_string(),
            component_type: ComponentType::New(to_json_binary(&true).unwrap()),
        }],
        name: String::from("Some App"),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        chain_info: None,
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();
    ADO_ADDRESSES
        .save(
            deps.as_mut().storage,
            "token",
            &Addr::unchecked("someaddress"),
        )
        .unwrap();

    let msg = ExecuteMsg::AddAppComponent {
        component: AppComponent {
            name: "token".to_string(),
            ado_type: "cw721".to_string(),
            component_type: ComponentType::New(to_json_binary(&true).unwrap()),
        },
    };

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(ContractError::NameAlreadyTaken {}, err);
}

#[test]
#[ignore = "Requires updating mock implementation for CosmWasm 2.0+"]
fn test_add_app_component() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);
    let inst_msg = InstantiateMsg {
        app_components: vec![],
        name: String::from("Some App"),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        chain_info: None,
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();

    let msg = ExecuteMsg::AddAppComponent {
        component: AppComponent {
            name: "token".to_string(),
            ado_type: "cw721".to_string(),
            component_type: ComponentType::New(to_json_binary(&true).unwrap()),
        },
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(1, res.messages.len());
    let inst_submsg: SubMsg = SubMsg {
        id: 1,
        msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
            code_id: 1,
            msg: to_json_binary(&true).unwrap(),
            funds: vec![],
            label: "Instantiate: cw721".to_string(),
            admin: Some("creator".to_string()),
        }),
        reply_on: ReplyOn::Always,
        gas_limit: None,
        payload: Binary::default(),
    };
    let expected = Response::new()
        .add_submessage(inst_submsg)
        .add_attributes(vec![
            attr("method", "add_app_component"),
            attr("name", "token"),
            attr("type", "cw721"),
        ]);

    assert_eq!(expected, res);

    assert_eq!(
        Addr::unchecked(""),
        ADO_ADDRESSES.load(deps.as_ref().storage, "token").unwrap()
    );
}

#[test]
fn test_claim_ownership_unauth() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);
    let inst_msg = InstantiateMsg {
        app_components: vec![],
        name: String::from("Some App"),
        owner: None,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        chain_info: None,
    };

    instantiate(deps.as_mut(), env.clone(), info, inst_msg).unwrap();
    let anyone = deps.api.addr_make("anyone");
    let unauth_info = message_info(&anyone, &[]);
    let msg = ExecuteMsg::ClaimOwnership {
        name: None,
        new_owner: None,
    };

    let err = execute(deps.as_mut(), env, unauth_info, msg).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err);
}

#[test]
fn test_claim_ownership_not_found() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);
    let inst_msg = InstantiateMsg {
        app_components: vec![],
        name: String::from("Some App"),
        owner: None,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        chain_info: None,
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();

    let msg = ExecuteMsg::ClaimOwnership {
        name: Some("token".to_string()),
        new_owner: None,
    };

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert!(matches!(err, ContractError::Std(StdError::NotFound { .. })));
}

#[test]
fn test_claim_ownership_empty() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);
    let inst_msg = InstantiateMsg {
        app_components: vec![],
        name: String::from("Some App"),
        owner: None,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        chain_info: None,
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();

    let msg = ExecuteMsg::ClaimOwnership {
        name: None,
        new_owner: None,
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_claim_ownership_all() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();

    env.contract.address = Addr::unchecked("owner");
    let sender = deps.api.addr_make("Sender");
    let info = message_info(&sender, &[]);

    let inst_msg = InstantiateMsg {
        app_components: vec![],
        name: String::from("Some App"),
        owner: None,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        chain_info: None,
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();

    let token_addr = deps.api.addr_make(MOCK_CW20_CONTRACT);
    let anchor_addr = deps.api.addr_make(MOCK_ANCHOR_CONTRACT);

    ADO_ADDRESSES
        .save(deps.as_mut().storage, "token", &token_addr)
        .unwrap();
    ADO_ADDRESSES
        .save(deps.as_mut().storage, "anchor", &anchor_addr)
        .unwrap();

    let execute_msg = ExecuteMsg::ClaimOwnership {
        name: None,      // None means claim ownership for all components
        new_owner: None, // None means use sender as the new owner
    };

    let result = execute(deps.as_mut(), env, info, execute_msg).unwrap();

    assert_eq!(2, result.messages.len());
}

#[test]
fn test_claim_ownership() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);
    let inst_msg = InstantiateMsg {
        app_components: vec![],
        name: String::from("Some App"),
        owner: None,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        chain_info: None,
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();
    let token_addr = deps.api.addr_make("tokenaddress");
    let anchor_addr = deps.api.addr_make("anchoraddress");
    ADO_ADDRESSES
        .save(deps.as_mut().storage, "token", &token_addr)
        .unwrap();
    ADO_ADDRESSES
        .save(deps.as_mut().storage, "anchor", &anchor_addr)
        .unwrap();

    let msg = ExecuteMsg::ClaimOwnership {
        name: Some("token".to_string()),
        new_owner: None,
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(1, res.messages.len());

    let exec_submsg: SubMsg = SubMsg {
        id: 200,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: token_addr.to_string(),
            msg: to_json_binary(&AndromedaMsg::Ownership(OwnershipMessage::UpdateOwner {
                new_owner: creator,
                expiration: None,
            }))
            .unwrap(),
            funds: vec![],
        }),
        reply_on: ReplyOn::Error,
        gas_limit: None,
        payload: Binary::default(),
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
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);
    let inst_msg = InstantiateMsg {
        app_components: vec![],
        name: String::from("Some App"),
        owner: None,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        chain_info: None,
    };

    instantiate(deps.as_mut(), env.clone(), info, inst_msg).unwrap();

    let anyone = deps.api.addr_make("anyone");
    let unauth_info = message_info(&anyone, &[]);
    let msg = ExecuteMsg::ProxyMessage {
        name: "token".to_string(),
        msg: to_json_binary(&true).unwrap(),
    };

    let err = execute(deps.as_mut(), env, unauth_info, msg).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err);
}

#[test]
fn test_proxy_message_not_found() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);
    let inst_msg = InstantiateMsg {
        app_components: vec![],
        name: String::from("Some App"),
        owner: None,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        chain_info: None,
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();

    let msg = ExecuteMsg::ProxyMessage {
        name: "token".to_string(),
        msg: to_json_binary(&true).unwrap(),
    };

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

    assert!(matches!(err, ContractError::Std(StdError::NotFound { .. })));
}

#[test]
fn test_proxy_message() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);
    let inst_msg = InstantiateMsg {
        app_components: vec![],
        name: String::from("Some App"),
        owner: None,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        chain_info: None,
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
        msg: to_json_binary(&true).unwrap(),
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let exec_submsg: SubMsg = SubMsg {
        id: 102,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "tokenaddress".to_string(),
            msg: to_json_binary(&true).unwrap(),
            funds: vec![],
        }),
        reply_on: ReplyOn::Error,
        gas_limit: None,
        payload: Binary::default(),
    };
    let expected = Response::new()
        .add_submessage(exec_submsg)
        .add_attributes(vec![
            attr("method", "app_message"),
            attr("recipient", "token"),
        ]);

    assert_eq!(expected, res)
}

#[test]
fn test_update_address_unauth() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);
    let inst_msg = InstantiateMsg {
        app_components: vec![],
        name: String::from("Some App"),
        owner: None,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        chain_info: None,
    };

    let token_addr = deps.api.addr_make("tokenaddress");
    ADO_ADDRESSES
        .save(deps.as_mut().storage, "token", &token_addr)
        .unwrap();
    instantiate(deps.as_mut(), env.clone(), info, inst_msg).unwrap();

    let anyone = deps.api.addr_make("anyone");
    let unauth_info = message_info(&anyone, &[]);
    let new_token_addr = deps.api.addr_make("newtokenaddress");
    let msg = ExecuteMsg::UpdateAddress {
        name: "token".to_string(),
        addr: new_token_addr.to_string(),
    };

    let err = execute(deps.as_mut(), env, unauth_info, msg).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err);
}

#[test]
fn test_update_address_not_found() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);
    let inst_msg = InstantiateMsg {
        app_components: vec![],
        name: String::from("Some App"),
        owner: None,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        chain_info: None,
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();

    let new_token_address = deps.api.addr_make("newtokenaddress");
    let msg = ExecuteMsg::UpdateAddress {
        name: "token".to_string(),
        addr: new_token_address.to_string(),
    };

    let res = execute(deps.as_mut(), env, info, msg);
    assert!(matches!(
        res,
        Err(ContractError::Std(StdError::NotFound { .. }))
    ));
}

#[test]
fn test_update_address() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);
    let inst_msg = InstantiateMsg {
        app_components: vec![],
        name: String::from("Some App"),
        owner: None,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        chain_info: None,
    };

    let token_address = deps.api.addr_make("tokenaddress");
    ADO_ADDRESSES
        .save(deps.as_mut().storage, "token", &token_address)
        .unwrap();
    instantiate(deps.as_mut(), env.clone(), info.clone(), inst_msg).unwrap();

    let new_token_address = deps.api.addr_make("newtokenaddress");
    let msg = ExecuteMsg::UpdateAddress {
        name: "token".to_string(),
        addr: new_token_address.to_string(),
    };

    execute(deps.as_mut(), env, info, msg).unwrap();

    let addr = ADO_ADDRESSES.load(deps.as_ref().storage, "token").unwrap();
    assert_eq!(new_token_address, addr)
}

#[test]
fn test_add_app_component_limit() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);

    let msg = InstantiateMsg {
        app_components: vec![],
        name: String::from("Some App"),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        chain_info: None,
    };

    // we can just call .unwrap() to assert this was a success
    instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mut i = 0;
    while i < 50 {
        i += 1;
        ADO_ADDRESSES
            .save(deps.as_mut().storage, &i.to_string(), &Addr::unchecked(""))
            .unwrap();
    }
    ADO_IDX.save(deps.as_mut().storage, &50).unwrap();

    let msg = ExecuteMsg::AddAppComponent {
        component: AppComponent {
            name: "token".to_string(),
            ado_type: "cw721".to_string(),
            component_type: ComponentType::New(to_json_binary(&true).unwrap()),
        },
    };

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(ContractError::TooManyAppComponents {}, err);
}

#[test]
#[allow(deprecated)]
fn test_reply_assign_app() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let mock_app_component = AppComponent {
        ado_type: "cw721".to_string(),
        name: "token".to_string(),
        component_type: ComponentType::New(to_json_binary(&true).unwrap()),
    };
    let cosmos_contract = deps.api.addr_make("cosmos2contract");
    let component_idx = 1;
    ADO_DESCRIPTORS
        .save(
            deps.as_mut().storage,
            &component_idx.to_string(),
            &mock_app_component,
        )
        .unwrap();
    ADO_ADDRESSES
        .save(
            deps.as_mut().storage,
            &mock_app_component.name,
            &cosmos_contract,
        )
        .unwrap();

    let mock_reply_event = Event::new("instantiate")
        .add_attribute("contract_address".to_string(), cosmos_contract.to_string());

    let mock_reply: Reply = Reply {
        id: ReplyId::AssignApp.repr(),
        result: SubMsgResult::Ok(SubMsgResponse {
            events: vec![mock_reply_event],
            msg_responses: vec![],
            data: None,
        }),
        gas_used: 0,
        payload: Binary::default(),
    };

    let res = reply(deps.as_mut(), env, mock_reply).unwrap();
    assert!(res.messages.is_empty());
}
