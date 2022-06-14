use cosmwasm_std::{
    coins,
    testing::{mock_env, mock_info},
    BankMsg, Decimal, Response, StdError,
};

use crate::contract::{execute, instantiate};
use andromeda_finance::splitter::{AddressPercent, ExecuteMsg, InstantiateMsg};
use andromeda_testing::testing::mock_querier::{
    mock_dependencies_custom, MOCK_ADDRESSLIST_CONTRACT,
};
use common::{
    ado_base::{
        modules::Module,
        recipient::{ADORecipient, Recipient},
        AndromedaMsg,
    },
    app::AndrAddress,
    error::ContractError,
};

#[test]
fn test_modules() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        modules: Some(vec![Module {
            module_type: "address_list".to_string(),
            is_mutable: false,
            address: AndrAddress {
                identifier: MOCK_ADDRESSLIST_CONTRACT.to_owned(),
            },
        }]),
        recipients: vec![AddressPercent {
            recipient: Recipient::from_string(String::from("Some Address")),
            percent: Decimal::percent(100),
        }],
        lock_time: Some(0),
    };
    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    let expected_res = Response::new()
        .add_attribute("action", "register_module")
        .add_attribute("module_idx", "1")
        .add_attribute("method", "instantiate")
        .add_attribute("type", "splitter");
    assert_eq!(expected_res, res);

    let msg = ExecuteMsg::Send {};
    let info = mock_info("anyone", &coins(100, "uusd"));

    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());

    assert_eq!(
        ContractError::Std(StdError::generic_err(
            "Querier contract error: InvalidAddress"
        ),),
        res.unwrap_err()
    );

    let info = mock_info("sender", &coins(100, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "Some Address".to_string(),
                amount: coins(100, "uusd"),
            })
            .add_attribute("action", "send")
            .add_attribute("sender", "sender"),
        res
    );
}

#[test]
fn test_update_app_contract() {
    let mut deps = mock_dependencies_custom(&[]);

    let modules: Vec<Module> = vec![Module {
        module_type: "address_list".to_string(),
        address: AndrAddress {
            identifier: MOCK_ADDRESSLIST_CONTRACT.to_owned(),
        },
        is_mutable: false,
    }];

    let info = mock_info("app_contract", &[]);
    let msg = InstantiateMsg {
        modules: Some(modules),
        recipients: vec![
            AddressPercent {
                recipient: Recipient::from_string(String::from("Some Address")),
                percent: Decimal::percent(50),
            },
            AddressPercent {
                recipient: Recipient::ADO(ADORecipient {
                    address: AndrAddress {
                        identifier: "e".to_string(),
                    },
                    msg: None,
                }),
                percent: Decimal::percent(50),
            },
        ],
        lock_time: Some(0),
    };

    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::UpdateAppContract {
        address: "app_contract".to_string(),
    });

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "update_app_contract")
            .add_attribute("address", "app_contract"),
        res
    );
}

#[test]
fn test_update_app_contract_invalid_recipient() {
    let mut deps = mock_dependencies_custom(&[]);

    let modules: Vec<Module> = vec![Module {
        module_type: "address_list".to_string(),
        address: AndrAddress {
            identifier: MOCK_ADDRESSLIST_CONTRACT.to_owned(),
        },
        is_mutable: false,
    }];

    let info = mock_info("app_contract", &[]);
    let msg = InstantiateMsg {
        modules: Some(modules),
        recipients: vec![AddressPercent {
            recipient: Recipient::ADO(ADORecipient {
                address: AndrAddress {
                    identifier: "z".to_string(),
                },
                msg: None,
            }),
            percent: Decimal::percent(100),
        }],
        lock_time: Some(0),
    };

    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::UpdateAppContract {
        address: "app_contract".to_string(),
    });

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidComponent {
            name: "z".to_string()
        },
        res.unwrap_err()
    );
}
