use crate::contract::instantiate;
use andromeda_std::os::kernel::InstantiateMsg;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg { owner: None };
    let env = mock_env();

    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

// #[test]
// fn parse_path_no_slash() {
//     let recipient = AndrAddr::from_string("user".to_string());
//     let message = to_binary(&"the_message").unwrap();
//     let storage = mock_dependencies();
//     let amp_msg = AMPMsg::new("recipient", message, None, None);
//     let res = parse_path(recipient, amp_msg, &storage.storage).unwrap();
//     assert_eq!(res, None)
// }

// #[test]
// fn parse_path_external_explicit() {
//     let mut deps = mock_dependencies();
//     let info = mock_info("creator", &[]);
//     let msg = InstantiateMsg { owner: None };
//     let env = mock_env();

//     let _ = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
//     let wormhole_address = Addr::unchecked("wormhole_address");
//     let msg = UpsertKeyAddress {
//         key: "wormhole".to_owned(),
//         value: wormhole_address.to_string(),
//     };
//     let _msg = execute(deps.as_mut(), env, info, msg).unwrap();

//     let recipient = AndrAddr::from_string("wormhole::/juno/user".to_string());
//     let message = to_binary(&"the_message").unwrap();
//     let storage = mock_dependencies();
//     let amp_msg = AMPMsg::new(
//         AndrAddr::from_string("recipient".to_string()),
//         message,
//         None,
//         None,
//     );
//     let _err = parse_path(recipient, amp_msg, &storage.storage).unwrap_err();
// }

// #[test]
// fn parse_path_unsupported_protocol() {
//     let mut deps = mock_dependencies();
//     let info = mock_info("creator", &[]);
//     let msg = InstantiateMsg { owner: None };
//     let env = mock_env();

//     let _ = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
//     let wormhole_address = Addr::unchecked("wormhole_address");
//     let msg = UpsertKeyAddress {
//         key: "wormhole".to_owned(),
//         value: wormhole_address.to_string(),
//     };
//     let _ = execute(deps.as_mut(), env, info, msg).unwrap();

//     let recipient = AndrAddr::from_string("eth::/juno/user".to_string());
//     let message = to_binary(&"the_message").unwrap();
//     let storage = mock_dependencies();
//     let amp_msg = AMPMsg::new("recipient", message, None, None);
//     let err = parse_path(recipient, amp_msg, &storage.storage).unwrap_err();
//     assert_eq!(err, ContractError::UnsupportedProtocol {})
// }

// #[test]
// fn parse_path_no_protocol_external() {
//     let mut deps = mock_dependencies();
//     let info = mock_info("creator", &[]);
//     let msg = InstantiateMsg { owner: None };
//     let env = mock_env();

//     let _ = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
//     let wormhole_address = Addr::unchecked("wormhole_address");
//     let msg = UpsertKeyAddress {
//         key: "wormhole".to_owned(),
//         value: wormhole_address.to_string(),
//     };
//     let _ = execute(deps.as_mut(), env, info, msg).unwrap();

//     let recipient = AndrAddr::from_string("juno/user".to_string());
//     let message = to_binary(&"the_message").unwrap();
//     let storage = mock_dependencies();
//     let amp_msg = AMPMsg::new("recipient", message, None, None);
//     let _err = parse_path(recipient, amp_msg, &storage.storage).unwrap_err();
// }

// #[test]
// fn parse_path_no_protocol_andromeda() {
//     let mut deps = mock_dependencies();
//     let info = mock_info("creator", &[]);
//     let msg = InstantiateMsg { owner: None };
//     let env = mock_env();

//     let _ = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
//     let wormhole_address = Addr::unchecked("wormhole_address");
//     let msg = UpsertKeyAddress {
//         key: "wormhole".to_owned(),
//         value: wormhole_address.to_string(),
//     };
//     let _ = execute(deps.as_mut(), env, info, msg).unwrap();

//     let recipient = AndrAddr::from_string("andromeda/user".to_string());
//     let message = to_binary(&"the_message").unwrap();
//     let storage = mock_dependencies();
//     let amp_msg = AMPMsg::new("recipient", message, None, None);
//     let res = parse_path(recipient, amp_msg, &storage.storage).unwrap();
//     assert!(res.is_none())
// }

// #[test]
// fn parse_path_no_protocol_no_chain() {
//     let mut deps = mock_dependencies();
//     let info = mock_info("creator", &[]);
//     let msg = InstantiateMsg { owner: None };
//     let env = mock_env();

//     let _ = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
//     let wormhole_address = Addr::unchecked("wormhole_address");
//     let msg = UpsertKeyAddress {
//         key: "wormhole".to_owned(),
//         value: wormhole_address.to_string(),
//     };
//     let _ = execute(deps.as_mut(), env, info, msg).unwrap();

//     let recipient = AndrAddr::from_string("/user".to_string());
//     let message = to_binary(&"the_message").unwrap();
//     let storage = mock_dependencies();
//     let amp_msg = AMPMsg::new("recipient", message, None, None);
//     let res = parse_path(recipient, amp_msg, &storage.storage).unwrap();
//     assert!(res.is_none())
// }
