use crate::contract::{execute, instantiate};
use andromeda_protocol::{
    swapper::{
        AssetInfo, Cw20HookMsg, ExecuteMsg, InstantiateMsg, SwapperCw20HookMsg, SwapperImpl,
        SwapperImplCw20HookMsg, SwapperImplExecuteMsg, SwapperMsg,
    },
    testing::mock_querier::{
        mock_dependencies_custom, MOCK_ASTROPORT_WRAPPER_CONTRACT, MOCK_CW20_CONTRACT,
        MOCK_CW20_CONTRACT2,
    },
};
use common::{
    ado_base::{modules::InstantiateType, recipient::Recipient},
    error::ContractError,
};
use cosmwasm_std::{
    coins,
    testing::{mock_env, mock_info},
    to_binary, Addr, BankMsg, CosmosMsg, DepsMut, ReplyOn, Response, SubMsg, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

fn init(deps: DepsMut) -> Response {
    let msg = InstantiateMsg {
        swapper_impl: SwapperImpl {
            instantiate_type: InstantiateType::Address(MOCK_ASTROPORT_WRAPPER_CONTRACT.to_owned()),
            name: "swapper_impl".to_string(),
        },
        primitive_contract: "primitive_contract".to_string(),
    };

    instantiate(deps, mock_env(), mock_info("sender", &[]), msg).unwrap()
}

#[test]
fn test_instantiate_swapper_impl_address() {
    let mut deps = mock_dependencies_custom(&[]);
    let res = init(deps.as_mut());

    assert_eq!(
        Response::new()
            .add_attribute("method", "instantiate")
            .add_attribute("type", "swapper"),
        res
    );
}

#[test]
fn test_instantiate_swapper_impl_new() {
    let mut deps = mock_dependencies_custom(&[]);
    let msg = InstantiateMsg {
        swapper_impl: SwapperImpl {
            instantiate_type: InstantiateType::New(to_binary(&"mock_instantiate_msg").unwrap()),
            name: "swapper_impl".to_string(),
        },
        primitive_contract: "primitive_contract".to_string(),
    };

    let res = instantiate(deps.as_mut(), mock_env(), mock_info("sender", &[]), msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("method", "instantiate")
            .add_attribute("type", "swapper")
            .add_submessage(SubMsg {
                id: 1,
                reply_on: ReplyOn::Always,
                msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                    admin: None,
                    code_id: 5,
                    msg: to_binary(&"mock_instantiate_msg").unwrap(),
                    funds: vec![],
                    label: "Instantiate: swapper_impl".to_string(),
                }),
                gas_limit: None,
            }),
        res
    );
}

#[test]
fn test_swap_native_same_asset() {
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::Swap {
        ask_asset_info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        recipient: None,
    };

    let info = mock_info("sender", &coins(100, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "swap")
            .add_submessage(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "sender".to_string(),
                amount: coins(100, "uusd")
            }))),
        res
    );
}

#[test]
fn test_swap_native_to_native() {
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::Swap {
        ask_asset_info: AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        },
        recipient: None,
    };

    let info = mock_info("sender", &coins(100, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let swap_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: MOCK_ASTROPORT_WRAPPER_CONTRACT.to_owned(),
        funds: info.funds.clone(),
        msg: to_binary(&SwapperImplExecuteMsg::Swapper(SwapperMsg::Swap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
        }))
        .unwrap(),
    });

    let send_execute_msg = ExecuteMsg::Send {
        ask_asset_info: AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        },
        recipient: Recipient::Addr("sender".to_string()),
    };

    let send_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: mock_env().contract.address.to_string(),
        funds: vec![],
        msg: to_binary(&send_execute_msg).unwrap(),
    });

    assert_eq!(
        Response::new()
            .add_attribute("action", "swap")
            .add_attribute("offer_denom", "uusd")
            .add_message(swap_msg)
            .add_message(send_msg),
        res
    );

    let res = execute(deps.as_mut(), mock_env(), info, send_execute_msg.clone());
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

    let info = mock_info(mock_env().contract.address.as_str(), &[]);

    // uusd exchanged for uluna.
    deps.querier
        .base
        .update_balance(mock_env().contract.address, coins(10, "uluna"));

    let res = execute(deps.as_mut(), mock_env(), info, send_execute_msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "send")
            .add_submessage(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "sender".to_string(),
                amount: coins(10, "uluna")
            }))),
        res
    );
}

#[test]
fn test_swap_native_to_cw20() {
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::Swap {
        ask_asset_info: AssetInfo::Token {
            contract_addr: Addr::unchecked(MOCK_CW20_CONTRACT),
        },
        recipient: None,
    };

    let info = mock_info("sender", &coins(100, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let swap_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: MOCK_ASTROPORT_WRAPPER_CONTRACT.to_owned(),
        funds: info.funds,
        msg: to_binary(&SwapperImplExecuteMsg::Swapper(SwapperMsg::Swap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            ask_asset_info: AssetInfo::Token {
                contract_addr: Addr::unchecked(MOCK_CW20_CONTRACT),
            },
        }))
        .unwrap(),
    });

    let send_execute_msg = ExecuteMsg::Send {
        ask_asset_info: AssetInfo::Token {
            contract_addr: Addr::unchecked(MOCK_CW20_CONTRACT),
        },
        recipient: Recipient::Addr("sender".to_string()),
    };

    let send_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: mock_env().contract.address.to_string(),
        funds: vec![],
        msg: to_binary(&send_execute_msg).unwrap(),
    });

    assert_eq!(
        Response::new()
            .add_attribute("action", "swap")
            .add_attribute("offer_denom", "uusd")
            .add_message(swap_msg)
            .add_message(send_msg),
        res
    );

    let info = mock_info(mock_env().contract.address.as_str(), &[]);
    let res = execute(deps.as_mut(), mock_env(), info, send_execute_msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "send")
            .add_submessage(SubMsg::new(WasmMsg::Execute {
                contract_addr: MOCK_CW20_CONTRACT.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    amount: 10u128.into(),
                    recipient: "sender".to_string()
                })
                .unwrap(),
                funds: vec![],
            })),
        res
    );
}

#[test]
fn test_swap_cw20_to_native() {
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "sender".to_string(),
        amount: 10u128.into(),
        msg: to_binary(&Cw20HookMsg::Swap {
            ask_asset_info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            recipient: None,
        })
        .unwrap(),
    });

    let info = mock_info(MOCK_CW20_CONTRACT, &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let swap_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: MOCK_CW20_CONTRACT.to_owned(),
        funds: vec![],
        msg: to_binary(&Cw20ExecuteMsg::Send {
            contract: MOCK_ASTROPORT_WRAPPER_CONTRACT.to_owned(),
            amount: 10u128.into(),
            msg: to_binary(&SwapperImplCw20HookMsg::Swapper(SwapperCw20HookMsg::Swap {
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
            }))
            .unwrap(),
        })
        .unwrap(),
    });
    let send_execute_msg = ExecuteMsg::Send {
        ask_asset_info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        recipient: Recipient::Addr("sender".to_string()),
    };

    let send_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: mock_env().contract.address.to_string(),
        funds: vec![],
        msg: to_binary(&send_execute_msg).unwrap(),
    });

    assert_eq!(
        Response::new()
            .add_attribute("action", "swap")
            .add_message(swap_msg)
            .add_message(send_msg),
        res
    );

    let res = execute(deps.as_mut(), mock_env(), info, send_execute_msg.clone());
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

    let info = mock_info(mock_env().contract.address.as_str(), &[]);

    // cw20 token exchanged for uluna.
    deps.querier
        .base
        .update_balance(mock_env().contract.address, coins(10, "uusd"));

    let res = execute(deps.as_mut(), mock_env(), info, send_execute_msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "send")
            .add_submessage(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "sender".to_string(),
                amount: coins(10, "uusd")
            }))),
        res
    );
}

#[test]
fn test_swap_cw20_same_asset() {
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "sender".to_string(),
        amount: 10u128.into(),
        msg: to_binary(&Cw20HookMsg::Swap {
            ask_asset_info: AssetInfo::Token {
                contract_addr: Addr::unchecked(MOCK_CW20_CONTRACT),
            },
            recipient: None,
        })
        .unwrap(),
    });

    let info = mock_info(MOCK_CW20_CONTRACT, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "swap")
            .add_submessage(SubMsg::new(WasmMsg::Execute {
                contract_addr: MOCK_CW20_CONTRACT.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    amount: 10u128.into(),
                    recipient: "sender".to_string()
                })
                .unwrap(),
                funds: vec![],
            })),
        res
    );
}

#[test]
fn test_swap_cw20_to_cw20() {
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "sender".to_string(),
        amount: 10u128.into(),
        msg: to_binary(&Cw20HookMsg::Swap {
            ask_asset_info: AssetInfo::Token {
                contract_addr: Addr::unchecked(MOCK_CW20_CONTRACT2),
            },
            recipient: None,
        })
        .unwrap(),
    });

    let info = mock_info(MOCK_CW20_CONTRACT, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let swap_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: MOCK_CW20_CONTRACT.to_owned(),
        funds: vec![],
        msg: to_binary(&Cw20ExecuteMsg::Send {
            contract: MOCK_ASTROPORT_WRAPPER_CONTRACT.to_owned(),
            amount: 10u128.into(),
            msg: to_binary(&SwapperImplCw20HookMsg::Swapper(SwapperCw20HookMsg::Swap {
                ask_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked(MOCK_CW20_CONTRACT2),
                },
            }))
            .unwrap(),
        })
        .unwrap(),
    });
    let send_execute_msg = ExecuteMsg::Send {
        ask_asset_info: AssetInfo::Token {
            contract_addr: Addr::unchecked(MOCK_CW20_CONTRACT2),
        },
        recipient: Recipient::Addr("sender".to_string()),
    };

    let send_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: mock_env().contract.address.to_string(),
        funds: vec![],
        msg: to_binary(&send_execute_msg).unwrap(),
    });

    assert_eq!(
        Response::new()
            .add_attribute("action", "swap")
            .add_message(swap_msg)
            .add_message(send_msg),
        res
    );

    let info = mock_info(mock_env().contract.address.as_str(), &[]);
    let res = execute(deps.as_mut(), mock_env(), info, send_execute_msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "send")
            .add_submessage(SubMsg::new(WasmMsg::Execute {
                contract_addr: MOCK_CW20_CONTRACT2.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    amount: 10u128.into(),
                    recipient: "sender".to_string()
                })
                .unwrap(),
                funds: vec![],
            })),
        res
    );
}
