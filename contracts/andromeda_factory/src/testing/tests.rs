use crate::contract::{execute, instantiate};
use crate::reply::REPLY_CREATE_TOKEN;

use andromeda_protocol::{
    factory::{ExecuteMsg, InstantiateMsg},
    modules::ModuleDefinition,
    modules::Rate,
    token::InstantiateMsg as TokenInstantiateMsg,
};
use cosmwasm_std::{
    attr,
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, ReplyOn, Response, SubMsg, WasmMsg,
};

static TOKEN_CODE_ID: u64 = 0;
const TOKEN_NAME: &str = "test";
const TOKEN_SYMBOL: &str = "TT";
const ADDRESS_LIST_CODE_ID: u64 = 1;
const RECEIPT_CODE_ID: u64 = 2;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        token_code_id: TOKEN_CODE_ID,
        address_list_code_id: ADDRESS_LIST_CODE_ID,
        receipt_code_id: RECEIPT_CODE_ID,
    };
    let env = mock_env();

    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_create() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);

    let whitelist_moderators = vec!["whitelist_moderator1".to_string()];
    let tax_fee: Rate = Rate::Percent(1u64);
    let tax_receivers = vec!["tax_recever1".to_string()];
    let royality_fee: Rate = Rate::Percent(1u64);
    let royality_receivers = vec!["royality_recever1".to_string()];
    let modules = vec![
        ModuleDefinition::Whitelist {
            moderators: Some(whitelist_moderators),
            address: None,
            code_id: Some(ADDRESS_LIST_CODE_ID),
        },
        ModuleDefinition::Taxable {
            rate: tax_fee,
            receivers: tax_receivers,
            description: None,
        },
        ModuleDefinition::Royalties {
            rate: royality_fee,
            receivers: royality_receivers,
            description: None,
        },
    ];

    let init_msg = InstantiateMsg {
        token_code_id: TOKEN_CODE_ID,
        receipt_code_id: RECEIPT_CODE_ID,
        address_list_code_id: ADDRESS_LIST_CODE_ID,
    };

    let res = instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let msg = ExecuteMsg::Create {
        name: TOKEN_NAME.to_string(),
        symbol: TOKEN_SYMBOL.to_string(),
        modules: modules.clone(),
    };

    let token_inst_msg = TokenInstantiateMsg {
        name: TOKEN_NAME.to_string(),
        symbol: TOKEN_SYMBOL.to_string(),
        minter: info.sender.to_string(),
        modules,
    };

    let inst_msg = WasmMsg::Instantiate {
        admin: Some(info.sender.to_string()),
        code_id: TOKEN_CODE_ID,
        funds: vec![],
        label: String::from("Address list instantiation"),
        msg: to_binary(&token_inst_msg).unwrap(),
    };

    let expected_msg = SubMsg {
        msg: inst_msg.into(),
        gas_limit: None,
        id: REPLY_CREATE_TOKEN,
        reply_on: ReplyOn::Always,
    };

    let expected_res = Response::new()
        .add_submessage(expected_msg)
        .add_attributes(vec![
            attr("action", "create"),
            attr("name", TOKEN_NAME.to_string()),
            attr("symbol", TOKEN_SYMBOL.to_string()),
        ]);

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(res, expected_res);
    assert_eq!(1, expected_res.messages.len())
}

// #[test]
// fn test_update_address() {
//     let creator = String::from("creator");
//     let mut deps = mock_dependencies(&[]);
//     let env = mock_env();
//     let info = mock_info(creator.clone().as_str(), &[]);

//     let msg = ExecuteMsg::TokenCreationHook {
//         symbol: TOKEN_SYMBOL.to_string(),
//         creator: creator.clone(),
//     };

//     let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

//     assert_eq!(res, Response::default());

//     let new_address = String::from("new");
//     let update_msg = ExecuteMsg::UpdateAddress {
//         symbol: TOKEN_SYMBOL.to_string(),
//         new_address: new_address.clone(),
//     };

//     let update_res = execute(deps.as_mut(), env.clone(), info.clone(), update_msg.clone()).unwrap();

//     assert_eq!(update_res, Response::default());

//     let query_msg = QueryMsg::GetAddress {
//         symbol: TOKEN_SYMBOL.to_string(),
//     };

//     let addr_res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
//     let addr_val: AddressResponse = from_binary(&addr_res).unwrap();

//     assert_eq!(new_address.clone(), addr_val.address);

//     let unauth_env = mock_env();
//     let unauth_info = mock_info("anyone", &[]);
//     let unauth_res = execute(
//         deps.as_mut(),
//         unauth_env.clone(),
//         unauth_info.clone(),
//         update_msg.clone(),
//     )
//     .unwrap_err();

//     assert_eq!(
//         unauth_res,
//         StdError::generic_err("Cannot update address for ADO that you did not create"),
//     );
// }
