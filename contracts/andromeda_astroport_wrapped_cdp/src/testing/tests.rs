use crate::contract::{execute, instantiate};
use andromeda_protocol::{
    astroport_wrapped_cdp::{ExecuteMsg, InstantiateMsg},
    swapper::{AssetInfo, SwapperMsg},
    testing::mock_querier::{
        mock_dependencies_custom, MOCK_ASTROPORT_FACTORY_CONTRACT, MOCK_ASTROPORT_ROUTER_CONTRACT,
    },
};
use astroport::router::{ExecuteMsg as AstroportRouterExecuteMsg, SwapOperation};
use cosmwasm_std::{
    coins,
    testing::{mock_env, mock_info},
    to_binary, Addr, CosmosMsg, DepsMut, Response, WasmMsg,
};

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
