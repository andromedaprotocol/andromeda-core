use crate::contract::{execute, instantiate, query};
use crate::state::{ADDRESS_LIST, IS_INCLUSIVE};
use crate::testing::mock_querier::{mock_dependencies_custom, MOCK_KERNEL_CONTRACT};
use andromeda_modules::address_list::{
    ExecuteMsg, IncludesAddressResponse, InstantiateMsg, QueryMsg,
};
use andromeda_std::ado_base::hooks::AndromedaHook;
use andromeda_std::ado_contract::ADOContract;

use andromeda_std::common::encode_binary;
use andromeda_std::error::ContractError;

use cosmwasm_std::{attr, from_json, DepsMut, MessageInfo, StdError};
use cosmwasm_std::{
    testing::{mock_env, mock_info},
    Response,
};

fn init(deps: DepsMut, info: MessageInfo) {
    instantiate(
        deps,
        mock_env(),
        info,
        InstantiateMsg {
            is_inclusive: true,
            kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
            owner: None,
        },
    )
    .unwrap();
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);

    init(deps.as_mut(), info);
}

#[test]
fn test_add_address() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let operator = "creator";
    let info = mock_info(operator, &[]);

    let address = "whitelistee";

    init(deps.as_mut(), info.clone());

    ADOContract::default()
        .execute_update_operators(deps.as_mut(), info.clone(), vec![operator.to_owned()])
        .unwrap();

    let msg = ExecuteMsg::AddAddress {
        address: address.to_string(),
    };

    //add address for registered operator

    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();
    let expected = Response::default().add_attributes(vec![
        attr("action", "add_address"),
        attr("address", address),
    ]);
    assert_eq!(expected, res);

    let whitelisted = ADDRESS_LIST.load(deps.as_ref().storage, address).unwrap();
    assert!(whitelisted);

    let included = ADDRESS_LIST.load(deps.as_ref().storage, "111").unwrap_err();

    match included {
        cosmwasm_std::StdError::NotFound { .. } => {}
        _ => {
            panic!();
        }
    }

    //add address for unregistered operator
    let unauth_info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), env, unauth_info, msg).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, res);
}

#[test]
fn test_add_addresses() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let operator = "creator";
    let info = mock_info(operator, &[]);

    let address = "whitelistee";
    let address_two = "whitlistee2";

    init(deps.as_mut(), info.clone());

    ADOContract::default()
        .execute_update_operators(deps.as_mut(), info.clone(), vec![operator.to_owned()])
        .unwrap();

    let msg = ExecuteMsg::AddAddresses { addresses: vec![] };

    //add address for registered operator

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(
        res,
        ContractError::Std(StdError::generic_err("addresses cannot be empty"))
    );

    let addresses = vec![address.to_string(), address_two.to_string()];
    let msg = ExecuteMsg::AddAddresses {
        addresses: addresses.clone(),
    };

    //add address for registered operator

    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();
    let expected = Response::default().add_attributes(vec![
        attr("action", "add_addresses"),
        attr("addresses", addresses.join(",")),
    ]);
    assert_eq!(expected, res);

    let whitelisted = ADDRESS_LIST.load(deps.as_ref().storage, address).unwrap();
    assert!(whitelisted);
    let whitelisted = ADDRESS_LIST
        .load(deps.as_ref().storage, address_two)
        .unwrap();
    assert!(whitelisted);

    let included = ADDRESS_LIST.load(deps.as_ref().storage, "111").unwrap_err();

    match included {
        cosmwasm_std::StdError::NotFound { .. } => {}
        _ => {
            panic!();
        }
    }

    //add address for unregistered operator
    let unauth_info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), env, unauth_info, msg).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, res);
}

#[test]
fn test_remove_address() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let operator = "creator";
    let info = mock_info(operator, &[]);

    let address = "whitelistee";

    init(deps.as_mut(), info.clone());

    //save operator
    ADOContract::default()
        .execute_update_operators(deps.as_mut(), info.clone(), vec![operator.to_owned()])
        .unwrap();

    let msg = ExecuteMsg::RemoveAddress {
        address: address.to_string(),
    };

    //add address for registered operator
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();
    let expected = Response::default().add_attributes(vec![
        attr("action", "remove_address"),
        attr("address", address),
    ]);
    assert_eq!(expected, res);

    let included_is_err = ADDRESS_LIST.load(deps.as_ref().storage, address).is_err();
    assert!(included_is_err);

    //add address for unregistered operator
    let unauth_info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), env, unauth_info, msg).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, res);
}

#[test]
fn test_execute_hook_whitelist() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let operator = "creator";
    let info = mock_info(operator, &[]);

    let address = "whitelistee";

    // Mark it as a whitelist.
    IS_INCLUSIVE.save(deps.as_mut().storage, &true).unwrap();
    init(deps.as_mut(), info.clone());

    let msg = ExecuteMsg::AddAddress {
        address: address.to_string(),
    };
    let _res = execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = QueryMsg::AndrHook(AndromedaHook::OnExecute {
        sender: address.to_string(),
        payload: encode_binary(&"".to_string()).unwrap(),
    });

    let res: Option<Response> = from_json(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(None, res);

    let msg = QueryMsg::AndrHook(AndromedaHook::OnExecute {
        sender: "random".to_string(),
        payload: encode_binary(&"".to_string()).unwrap(),
    });

    let res_err: ContractError = query(deps.as_ref(), mock_env(), msg).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, res_err);
}

#[test]
fn test_execute_hook_blacklist() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let operator = "creator";
    let info = mock_info(operator, &[]);

    let address = "blacklistee";
    init(deps.as_mut(), info.clone());

    // Mark it as a blacklist.
    IS_INCLUSIVE.save(deps.as_mut().storage, &false).unwrap();
    ADOContract::default()
        .execute_update_operators(deps.as_mut(), info.clone(), vec![operator.to_owned()])
        .unwrap();

    let msg = ExecuteMsg::AddAddress {
        address: address.to_string(),
    };
    let _res = execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = QueryMsg::AndrHook(AndromedaHook::OnExecute {
        sender: "random".to_string(),
        payload: encode_binary(&"".to_string()).unwrap(),
    });

    let res: Option<Response> = from_json(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(None, res);

    let msg = QueryMsg::AndrHook(AndromedaHook::OnExecute {
        sender: address.to_string(),
        payload: encode_binary(&"".to_string()).unwrap(),
    });

    let res_err: ContractError = query(deps.as_ref(), mock_env(), msg).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, res_err);
}

#[test]
fn test_andr_get_query() {
    let mut deps = mock_dependencies_custom(&[]);

    let address = "whitelistee";

    ADDRESS_LIST
        .save(deps.as_mut().storage, address, &true)
        .unwrap();

    let msg = QueryMsg::IncludesAddress {
        address: address.to_owned(),
    };

    let res: IncludesAddressResponse =
        from_json(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();

    assert_eq!(IncludesAddressResponse { included: true }, res);
}
