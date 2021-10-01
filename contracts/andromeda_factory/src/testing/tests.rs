use crate::state::read_creator;
use crate::contract::{ instantiate, query, execute };

use cosmwasm_std::{ StdError, from_binary, Response, to_binary, WasmMsg, };
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use andromeda_protocol::modules::Rate;
use andromeda_protocol::{
    factory::{AddressResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    hook::InitHook,
    token::InstantiateMsg as TokenInstantiateMsg,
    modules::ModuleDefinition,
};

static TOKEN_CODE_ID: u64 = 0;
const TOKEN_NAME: &str = "test";
const TOKEN_SYMBOL: &str = "T";

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        token_code_id: TOKEN_CODE_ID,
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
    let tax_fee:Rate = Rate::Percent(1u64);
    let tax_receivers = vec!["tax_recever1".to_string()];
    let royality_fee:Rate = Rate::Percent(1u64);
    let royality_receivers = vec!["royality_recever1".to_string()];
    let size_limit = 100u64;
    let modules = vec![
        ModuleDefinition::Whitelist{
            moderators: whitelist_moderators
        },
        ModuleDefinition::Taxable{
            rate: tax_fee,
            receivers: tax_receivers,
            description: None,
        },
        ModuleDefinition::Royalties{
            rate: royality_fee,
            receivers: royality_receivers,
            description: None,
        },
        ModuleDefinition::MetadataStorage {
            size_limit: Some(size_limit),
            description: None,
        },
    ];

    let init_msg = InstantiateMsg {
        token_code_id: TOKEN_CODE_ID,
    };

    let res = instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let msg = ExecuteMsg::Create {
        name: TOKEN_NAME.to_string(),
        symbol: TOKEN_SYMBOL.to_string(),
        modules: modules.clone(),
        metadata_limit: None,
    };

    let expected_msg = WasmMsg::Instantiate {
        code_id: TOKEN_CODE_ID,
        funds: vec![],
        label: "".to_string(),
        admin: None,
        msg: to_binary(&TokenInstantiateMsg {
            name: TOKEN_NAME.to_string(),
            symbol: TOKEN_SYMBOL.to_string(),
            minter: String::from("creator"),
            modules: modules.clone(),
            init_hook: Some(InitHook {
                msg: to_binary(&ExecuteMsg::TokenCreationHook {
                    symbol: TOKEN_SYMBOL.to_string(),
                    creator: String::from("creator"),
                })
                .unwrap(),
                contract_addr: info.sender.to_string(),
            }),
            metadata_limit: None,
        })
        .unwrap(),
    };

    let expected_res = Response::new().add_message(expected_msg);

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(res, expected_res);
    assert_eq!(1, expected_res.messages.len())
}

#[test]
fn test_token_creation() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);

    let msg = ExecuteMsg::TokenCreationHook {
        symbol: TOKEN_SYMBOL.to_string(),
        creator: String::from("creator"),
    };

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    assert_eq!(res, Response::default());

    let query_msg = QueryMsg::GetAddress {
        symbol: TOKEN_SYMBOL.to_string(),
    };

    let addr_res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let addr_val: AddressResponse = from_binary(&addr_res).unwrap();

    assert_eq!(info.sender, addr_val.address);
    let creator = match read_creator(&deps.storage, TOKEN_SYMBOL.to_string()) {
        Ok(addr) => addr,
        _ => String::default(),
    };
    assert_eq!(info.sender, creator)
}

#[test]
fn test_update_address() {
    let creator = String::from("creator");
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let info = mock_info(creator.clone().as_str(), &[]);

    let msg = ExecuteMsg::TokenCreationHook {
        symbol: TOKEN_SYMBOL.to_string(),
        creator: creator.clone(),
    };

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    assert_eq!(res, Response::default());

    let new_address = String::from("new");
    let update_msg = ExecuteMsg::UpdateAddress {
        symbol: TOKEN_SYMBOL.to_string(),
        new_address: new_address.clone(),
    };

    let update_res =
        execute(deps.as_mut(), env.clone(), info.clone(), update_msg.clone()).unwrap();

    assert_eq!(update_res, Response::default());

    let query_msg = QueryMsg::GetAddress {
        symbol: TOKEN_SYMBOL.to_string(),
    };

    let addr_res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let addr_val: AddressResponse = from_binary(&addr_res).unwrap();

    assert_eq!(new_address.clone(), addr_val.address);

    let unauth_env = mock_env();
    let unauth_info = mock_info("anyone", &[]);
    let unauth_res = execute(
        deps.as_mut(),
        unauth_env.clone(),
        unauth_info.clone(),
        update_msg.clone(),
    )
    .unwrap_err();

    assert_eq!(
        unauth_res,
        StdError::generic_err("Cannot update address for ADO that you did not create"),
    );
}

