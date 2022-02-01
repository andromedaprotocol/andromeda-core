use crate::contract::{execute, instantiate};
use andromeda_protocol::{
    communication::modules::{InstantiateType, Module, ModuleType},
    cw20::{ExecuteMsg, InstantiateMsg},
    testing::mock_querier::{
        mock_dependencies_custom, MOCK_ADDRESSLIST_CONTRACT, MOCK_RATES_CONTRACT,
        MOCK_RECEIPT_CONTRACT,
    },
};
use cosmwasm_std::{
    attr, coin,
    testing::{mock_env, mock_info},
    to_binary, BankMsg, CosmosMsg, Event, ReplyOn, Response, SubMsg, Uint128, WasmMsg,
};
use cw20::Cw20Coin;

#[test]
fn test_transfer() {
    // TODO: Test InstantiateType::New() when Fetch contract works.
    let modules: Vec<Module> = vec![
        Module {
            module_type: ModuleType::Rates,
            instantiate: InstantiateType::Address(MOCK_RATES_CONTRACT.into()),
        },
        Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address(MOCK_ADDRESSLIST_CONTRACT.into()),
        },
        Module {
            module_type: ModuleType::Receipt,
            instantiate: InstantiateType::Address(MOCK_RECEIPT_CONTRACT.into()),
        },
    ];

    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("sender", &[]);

    let instantiate_msg = InstantiateMsg {
        name: "Name".into(),
        symbol: "Symbol".into(),
        decimals: 6,
        initial_balances: vec![Cw20Coin {
            amount: 1000u128.into(),
            address: "sender".to_string(),
        }],
        mint: None,
        marketing: None,
        modules: Some(modules),
    };

    let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
    assert_eq!(Response::default(), res);
}
