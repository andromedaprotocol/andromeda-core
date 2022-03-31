use cosmwasm_std::{
    coins,
    testing::{mock_env, mock_info},
    BankMsg, Decimal, Response, StdError,
};

use crate::contract::{execute, instantiate};
use andromeda_protocol::{
    splitter::{AddressPercent, ExecuteMsg, InstantiateMsg},
    testing::mock_querier::{mock_dependencies_custom, MOCK_ADDRESSLIST_CONTRACT},
};
use common::{
    ado_base::modules::Module, ado_base::recipient::Recipient, error::ContractError,
    mission::AndrAddress,
};

#[test]
fn test_address_list() {
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
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

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
