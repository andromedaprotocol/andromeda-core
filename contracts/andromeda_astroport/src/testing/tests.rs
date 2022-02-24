use crate::contract::{execute, instantiate};
use andromeda_protocol::{
    astroport::{Cw20HookMsg, ExecuteMsg, InstantiateMsg},
    swapper::{AssetInfo, SwapperCw20HookMsg, SwapperMsg},
    testing::mock_querier::{
        mock_dependencies_custom, MOCK_ASTROPORT_FACTORY_CONTRACT, MOCK_ASTROPORT_ROUTER_CONTRACT,
    },
};
use astroport::router::{
    Cw20HookMsg as AstroportRouterCw20HookMsg, ExecuteMsg as AstroportRouterExecuteMsg,
    SwapOperation,
};
use cosmwasm_std::{
    coins,
    testing::{mock_env, mock_info},
    to_binary, Addr, CosmosMsg, DepsMut, Response, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

fn init(deps: DepsMut) {
    let msg = InstantiateMsg {
        astroport_factory_contract: MOCK_ASTROPORT_FACTORY_CONTRACT.to_owned(),
        astroport_staking_contract: "staking".to_string(),
        astroport_maker_contract: "maker".to_string(),
        astroport_router_contract: MOCK_ASTROPORT_ROUTER_CONTRACT.to_owned(),
        astroport_vesting_contract: "vesting".to_string(),
    };

    let _res = instantiate(deps, mock_env(), mock_info("sender", &[]), msg).unwrap();
}

#[test]
fn test_native_swap() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::Swapper(SwapperMsg::Swap {
        offer_asset_info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        ask_asset_info: AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        },
    });

    let info = mock_info("sender", &coins(100, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let swap_msg = AstroportRouterExecuteMsg::ExecuteSwapOperations {
        operations: vec![SwapOperation::NativeSwap {
            offer_denom: "uusd".to_string(),
            ask_denom: "uluna".to_string(),
        }],
        minimum_receive: None,
        to: Some(info.sender.clone()),
    };
    assert_eq!(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_ASTROPORT_ROUTER_CONTRACT.to_owned(),
            funds: info.funds,
            msg: to_binary(&swap_msg).unwrap()
        })),
        res
    );
}

#[test]
fn test_native_to_token() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::Swapper(SwapperMsg::Swap {
        offer_asset_info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        ask_asset_info: AssetInfo::Token {
            contract_addr: Addr::unchecked("token".to_string()),
        },
    });

    let info = mock_info("sender", &coins(100, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let swap_msg = AstroportRouterExecuteMsg::ExecuteSwapOperations {
        operations: vec![SwapOperation::AstroSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            }
            .into(),
            ask_asset_info: AssetInfo::Token {
                contract_addr: Addr::unchecked("token".to_string()),
            }
            .into(),
        }],
        minimum_receive: None,
        to: Some(info.sender.clone()),
    };
    assert_eq!(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_ASTROPORT_ROUTER_CONTRACT.to_owned(),
            funds: info.funds,
            msg: to_binary(&swap_msg).unwrap()
        })),
        res
    );
}

#[test]
fn test_native_to_native_to_token() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::Swapper(SwapperMsg::Swap {
        offer_asset_info: AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        },
        ask_asset_info: AssetInfo::Token {
            contract_addr: Addr::unchecked("token".to_string()),
        },
    });

    let info = mock_info("sender", &coins(100, "uluna"));
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let swap_msg = AstroportRouterExecuteMsg::ExecuteSwapOperations {
        operations: vec![
            SwapOperation::NativeSwap {
                offer_denom: "uluna".to_string(),
                ask_denom: "uusd".to_string(),
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                }
                .into(),
                ask_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("token".to_string()),
                }
                .into(),
            },
        ],
        minimum_receive: None,
        to: Some(info.sender.clone()),
    };
    assert_eq!(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_ASTROPORT_ROUTER_CONTRACT.to_owned(),
            funds: info.funds,
            msg: to_binary(&swap_msg).unwrap()
        })),
        res
    );
}

#[test]
fn test_token_to_native() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let hook_msg = Cw20HookMsg::Swapper(SwapperCw20HookMsg::Swap {
        ask_asset_info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
    });

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "sender".to_string(),
        amount: 100u128.into(),
        msg: to_binary(&hook_msg).unwrap(),
    });
    let token_addr = "token_addr";

    let info = mock_info(token_addr, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let swap_msg = AstroportRouterCw20HookMsg::ExecuteSwapOperations {
        operations: vec![SwapOperation::AstroSwap {
            offer_asset_info: AssetInfo::Token {
                contract_addr: Addr::unchecked("token_addr"),
            }
            .into(),
            ask_asset_info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            }
            .into(),
        }],
        minimum_receive: None,
        to: Some("sender".to_string()),
    };
    let msg = Cw20ExecuteMsg::Send {
        contract: MOCK_ASTROPORT_ROUTER_CONTRACT.to_owned(),
        amount: 100u128.into(),
        msg: to_binary(&swap_msg).unwrap(),
    };
    assert_eq!(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: token_addr.to_string(),
            funds: vec![],
            msg: to_binary(&msg).unwrap()
        })),
        res
    );
}

#[test]
fn test_token_to_native_to_token() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let offer_token = "offer_token";
    let ask_token = "ask_token";
    let hook_msg = Cw20HookMsg::Swapper(SwapperCw20HookMsg::Swap {
        ask_asset_info: AssetInfo::Token {
            contract_addr: Addr::unchecked(ask_token),
        },
    });

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "sender".to_string(),
        amount: 100u128.into(),
        msg: to_binary(&hook_msg).unwrap(),
    });

    let info = mock_info(offer_token, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let swap_msg = AstroportRouterCw20HookMsg::ExecuteSwapOperations {
        operations: vec![
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked(offer_token),
                }
                .into(),
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                }
                .into(),
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                }
                .into(),
                ask_asset_info: AssetInfo::Token {
                    contract_addr: Addr::unchecked(ask_token),
                }
                .into(),
            },
        ],
        minimum_receive: None,
        to: Some("sender".to_string()),
    };
    let msg = Cw20ExecuteMsg::Send {
        contract: MOCK_ASTROPORT_ROUTER_CONTRACT.to_owned(),
        amount: 100u128.into(),
        msg: to_binary(&swap_msg).unwrap(),
    };
    assert_eq!(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: offer_token.to_string(),
            funds: vec![],
            msg: to_binary(&msg).unwrap()
        })),
        res
    );
}
