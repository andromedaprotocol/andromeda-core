use cosmwasm_std::{
    coins,
    testing::{mock_env, mock_info},
    to_binary, Addr, BankMsg, CosmosMsg, DepsMut, Response, SubMsg, Uint128, WasmMsg,
};

use crate::{
    contract::{execute, instantiate},
    primitive_keys::{
        ASTROPORT_ASTRO, ASTROPORT_FACTORY, ASTROPORT_GENERATOR, ASTROPORT_ROUTER,
        ASTROPORT_STAKING, ASTROPORT_XASTRO,
    },
    testing::mock_querier::{
        mock_dependencies_custom, MOCK_ASTROPORT_FACTORY_CONTRACT,
        MOCK_ASTROPORT_GENERATOR_CONTRACT, MOCK_ASTROPORT_PAIR_CONTRACT,
        MOCK_ASTROPORT_ROUTER_CONTRACT, MOCK_ASTROPORT_STAKING_CONTRACT, MOCK_ASTRO_TOKEN,
        MOCK_LP_ASSET1, MOCK_LP_ASSET2, MOCK_LP_TOKEN_CONTRACT, MOCK_PRIMITIVE_CONTRACT,
        MOCK_XASTRO_TOKEN,
    },
};
use ado_base::ADOContract;
use andromeda_protocol::{
    astroport::{Cw20HookMsg, ExecuteMsg, InstantiateMsg},
    swapper::{SwapperCw20HookMsg, SwapperMsg},
};
use astroport::{
    generator::{Cw20HookMsg as GeneratorCw20HookMsg, ExecuteMsg as GeneratorExecuteMsg},
    pair::{Cw20HookMsg as PairCw20HookMsg, ExecuteMsg as AstroportPairExecuteMsg},
    router::{
        Cw20HookMsg as AstroportRouterCw20HookMsg, ExecuteMsg as AstroportRouterExecuteMsg,
        SwapOperation,
    },
    staking::Cw20HookMsg as StakingCw20HookMsg,
};
use common::error::ContractError;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_asset::{Asset, AssetInfo};

fn init(deps: DepsMut) -> Response {
    let msg = InstantiateMsg {
        primitive_contract: MOCK_PRIMITIVE_CONTRACT.to_owned(),
    };
    instantiate(deps, mock_env(), mock_info("sender", &[]), msg).unwrap()
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut());
    let contract = ADOContract::default();

    assert_eq!(
        MOCK_XASTRO_TOKEN,
        contract
            .get_cached_address(deps.as_ref().storage, ASTROPORT_XASTRO)
            .unwrap()
    );
    assert_eq!(
        MOCK_ASTRO_TOKEN,
        contract
            .get_cached_address(deps.as_ref().storage, ASTROPORT_ASTRO)
            .unwrap()
    );
    assert_eq!(
        MOCK_ASTROPORT_ROUTER_CONTRACT,
        contract
            .get_cached_address(deps.as_ref().storage, ASTROPORT_ROUTER)
            .unwrap()
    );
    assert_eq!(
        MOCK_ASTROPORT_FACTORY_CONTRACT,
        contract
            .get_cached_address(deps.as_ref().storage, ASTROPORT_FACTORY)
            .unwrap()
    );
    assert_eq!(
        MOCK_ASTROPORT_STAKING_CONTRACT,
        contract
            .get_cached_address(deps.as_ref().storage, ASTROPORT_STAKING)
            .unwrap()
    );
    assert_eq!(
        MOCK_ASTROPORT_GENERATOR_CONTRACT,
        contract
            .get_cached_address(deps.as_ref().storage, ASTROPORT_GENERATOR)
            .unwrap()
    );
}

#[test]
fn test_native_swap() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::Swapper(SwapperMsg::Swap {
        offer_asset_info: AssetInfo::native("uusd"),
        ask_asset_info: AssetInfo::native("uluna"),
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
        offer_asset_info: AssetInfo::native("uusd"),
        ask_asset_info: AssetInfo::Cw20(Addr::unchecked("token")),
    });

    let info = mock_info("sender", &coins(100, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let swap_msg = AstroportRouterExecuteMsg::ExecuteSwapOperations {
        operations: vec![SwapOperation::AstroSwap {
            offer_asset_info: AssetInfo::native("uusd").into(),
            ask_asset_info: AssetInfo::Cw20(Addr::unchecked("token")).into(),
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
        offer_asset_info: AssetInfo::native("uluna"),
        ask_asset_info: AssetInfo::Cw20(Addr::unchecked("token")),
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
                offer_asset_info: AssetInfo::native("uusd").into(),
                ask_asset_info: AssetInfo::Cw20(Addr::unchecked("token")).into(),
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
        ask_asset_info: AssetInfo::native("uusd"),
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
            offer_asset_info: AssetInfo::Cw20(Addr::unchecked("token_addr")).into(),
            ask_asset_info: AssetInfo::native("uusd").into(),
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
        ask_asset_info: AssetInfo::Cw20(Addr::unchecked(ask_token)),
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
                offer_asset_info: AssetInfo::Cw20(Addr::unchecked(offer_token)).into(),
                ask_asset_info: AssetInfo::native("uusd").into(),
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::native("uusd").into(),
                ask_asset_info: AssetInfo::Cw20(Addr::unchecked(ask_token)).into(),
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

#[test]
fn test_provide_liquidity_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    let contract = ADOContract::default();
    init(deps.as_mut());
    assert!(contract
        .withdrawable_tokens
        .has(deps.as_mut().storage, MOCK_ASTRO_TOKEN));

    let assets = [
        Asset {
            info: AssetInfo::Cw20(Addr::unchecked(MOCK_LP_ASSET1)),
            amount: 100u128.into(),
        },
        Asset {
            info: AssetInfo::Cw20(Addr::unchecked(MOCK_LP_ASSET2)),
            amount: 200u128.into(),
        },
    ];

    let msg = ExecuteMsg::ProvideLiquidity {
        assets: assets.map(|a| a),
        slippage_tolerance: None,
        auto_stake: None,
    };

    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_provide_liquidity_cw20_cw20() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("sender", &[]);

    init(deps.as_mut());
    assert!(ADOContract::default()
        .withdrawable_tokens
        .has(deps.as_mut().storage, MOCK_ASTRO_TOKEN));

    let assets = [
        Asset {
            info: AssetInfo::Cw20(Addr::unchecked(MOCK_LP_ASSET1)),
            amount: 100u128.into(),
        },
        Asset {
            info: AssetInfo::Cw20(Addr::unchecked(MOCK_LP_ASSET2)),
            amount: 200u128.into(),
        },
    ];

    let msg = ExecuteMsg::ProvideLiquidity {
        assets,
        slippage_tolerance: None,
        auto_stake: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    assert!(ADOContract::default()
        .withdrawable_tokens
        .has(deps.as_ref().storage, MOCK_LP_TOKEN_CONTRACT));

    assert_eq!(
        Response::new()
            .add_submessage(SubMsg::new(WasmMsg::Execute {
                contract_addr: MOCK_LP_ASSET1.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: mock_env().contract.address.to_string(),
                    // Reduced amount being transfered.
                    amount: 50u128.into(),
                })
                .unwrap(),
                funds: vec![],
            }))
            .add_submessage(SubMsg::new(WasmMsg::Execute {
                contract_addr: MOCK_LP_ASSET1.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: MOCK_ASTROPORT_PAIR_CONTRACT.to_owned(),
                    amount: 50u128.into(),
                    expires: None,
                })
                .unwrap(),
                funds: vec![],
            }))
            .add_submessage(SubMsg::new(WasmMsg::Execute {
                contract_addr: MOCK_LP_ASSET2.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: mock_env().contract.address.to_string(),
                    amount: 200u128.into(),
                })
                .unwrap(),
                funds: vec![],
            }))
            .add_submessage(SubMsg::new(WasmMsg::Execute {
                contract_addr: MOCK_LP_ASSET2.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: MOCK_ASTROPORT_PAIR_CONTRACT.to_owned(),
                    amount: 200u128.into(),
                    expires: None,
                })
                .unwrap(),
                funds: vec![],
            }))
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_ASTROPORT_PAIR_CONTRACT.to_owned(),
                msg: to_binary(&AstroportPairExecuteMsg::ProvideLiquidity {
                    assets: [
                        Asset {
                            info: AssetInfo::Cw20(Addr::unchecked(MOCK_LP_ASSET1)),
                            // Reduced amount being sent.
                            amount: 50u128.into(),
                        }
                        .into(),
                        Asset {
                            info: AssetInfo::Cw20(Addr::unchecked(MOCK_LP_ASSET2)),
                            amount: 200u128.into(),
                        }
                        .into()
                    ],
                    slippage_tolerance: None,
                    auto_stake: None,
                    receiver: Some(mock_env().contract.address.to_string()),
                })
                .unwrap(),
                funds: vec![],
            }))
            .add_attribute("action", "provide_liquidity"),
        res
    );
}

#[test]
fn test_provide_liquidity_native_cw20() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("sender", &coins(100, "uusd"));

    init(deps.as_mut());
    assert!(ADOContract::default()
        .withdrawable_tokens
        .has(deps.as_mut().storage, MOCK_ASTRO_TOKEN));

    let assets = [
        Asset {
            info: AssetInfo::native("uusd"),
            amount: 100u128.into(),
        },
        Asset {
            info: AssetInfo::Cw20(Addr::unchecked(MOCK_LP_ASSET2)),
            amount: 200u128.into(),
        },
    ];

    let msg = ExecuteMsg::ProvideLiquidity {
        assets,
        slippage_tolerance: None,
        auto_stake: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    assert!(ADOContract::default()
        .withdrawable_tokens
        .has(deps.as_ref().storage, MOCK_LP_TOKEN_CONTRACT));

    assert_eq!(
        Response::new()
            .add_submessage(SubMsg::new(WasmMsg::Execute {
                contract_addr: MOCK_LP_ASSET2.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: mock_env().contract.address.to_string(),
                    amount: 200u128.into(),
                })
                .unwrap(),
                funds: vec![],
            }))
            .add_submessage(SubMsg::new(WasmMsg::Execute {
                contract_addr: MOCK_LP_ASSET2.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: MOCK_ASTROPORT_PAIR_CONTRACT.to_owned(),
                    amount: 200u128.into(),
                    expires: None,
                })
                .unwrap(),
                funds: vec![],
            }))
            .add_submessage(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "sender".to_string(),
                amount: coins(50, "uusd"),
            })))
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_ASTROPORT_PAIR_CONTRACT.to_owned(),
                msg: to_binary(&AstroportPairExecuteMsg::ProvideLiquidity {
                    assets: [
                        Asset {
                            info: AssetInfo::native("uusd"),
                            amount: 50u128.into(),
                        }
                        .into(),
                        Asset {
                            info: AssetInfo::Cw20(Addr::unchecked(MOCK_LP_ASSET2)),
                            amount: 200u128.into(),
                        }
                        .into()
                    ],
                    slippage_tolerance: None,
                    auto_stake: None,
                    receiver: Some(mock_env().contract.address.to_string()),
                })
                .unwrap(),
                funds: coins(50, "uusd"),
            }))
            .add_attribute("action", "provide_liquidity"),
        res
    );
}

#[test]
fn test_withdraw_liquidity() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("sender", &[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::WithdrawLiquidity {
        pair_address: MOCK_ASTROPORT_PAIR_CONTRACT.to_owned(),
        amount: None,
        recipient: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_LP_TOKEN_CONTRACT.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: MOCK_ASTROPORT_PAIR_CONTRACT.to_owned(),
                    amount: 10u128.into(),
                    msg: to_binary(&PairCw20HookMsg::WithdrawLiquidity {}).unwrap(),
                })
                .unwrap(),
                funds: vec![],
            })))
            .add_submessage(SubMsg::new(WasmMsg::Execute {
                contract_addr: MOCK_LP_ASSET1.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "sender".to_string(),
                    amount: 10u128.into(),
                })
                .unwrap(),
                funds: vec![],
            }))
            .add_submessage(SubMsg::new(WasmMsg::Execute {
                contract_addr: MOCK_LP_ASSET2.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "sender".to_string(),
                    amount: 20u128.into(),
                })
                .unwrap(),
                funds: vec![],
            }))
            .add_attribute("action", "withdraw_liquidity"),
        res
    );
}

#[test]
fn test_withdraw_liquidity_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::WithdrawLiquidity {
        pair_address: MOCK_ASTROPORT_PAIR_CONTRACT.to_owned(),
        amount: None,
        recipient: None,
    };

    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_stake_lp() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("sender", &[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::StakeLp {
        lp_token_contract: MOCK_LP_TOKEN_CONTRACT.to_owned(),
        amount: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_LP_TOKEN_CONTRACT.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: MOCK_ASTROPORT_GENERATOR_CONTRACT.to_owned(),
                    amount: 10u128.into(),
                    msg: to_binary(&GeneratorCw20HookMsg::Deposit {}).unwrap(),
                })
                .unwrap(),
                funds: vec![],
            }))
            .add_attribute("action", "stake_lp")
            .add_attribute("amount", "10")
            .add_attribute("lp_token", MOCK_LP_TOKEN_CONTRACT),
        res
    );
}

#[test]
fn test_stake_lp_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::StakeLp {
        lp_token_contract: MOCK_LP_TOKEN_CONTRACT.to_owned(),
        amount: None,
    };

    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_unstake_lp() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("sender", &[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::UnstakeLp {
        lp_token_contract: MOCK_LP_TOKEN_CONTRACT.to_owned(),
        amount: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_ASTROPORT_GENERATOR_CONTRACT.to_owned(),
                funds: vec![],
                msg: to_binary(&GeneratorExecuteMsg::Withdraw {
                    amount: 10u128.into(),
                    lp_token: Addr::unchecked(MOCK_LP_TOKEN_CONTRACT)
                })
                .unwrap(),
            }))
            .add_attribute("action", "unstake_lp")
            .add_attribute("amount", "10")
            .add_attribute("lp_token", MOCK_LP_TOKEN_CONTRACT),
        res
    );
}

#[test]
fn test_unstake_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::UnstakeLp {
        lp_token_contract: MOCK_LP_TOKEN_CONTRACT.to_owned(),
        amount: None,
    };

    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_claim_lp_staking_rewards() {
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::ClaimLpStakingRewards {
        lp_token_contract: MOCK_LP_TOKEN_CONTRACT.to_owned(),
        auto_stake: None,
    };

    let info = mock_info("sender", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "claim_lp_staking_rewards")
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_ASTROPORT_GENERATOR_CONTRACT.to_owned(),
                funds: vec![],
                msg: to_binary(&GeneratorExecuteMsg::Withdraw {
                    amount: Uint128::zero(),
                    lp_token: Addr::unchecked(MOCK_LP_TOKEN_CONTRACT),
                })
                .unwrap(),
            })),
        res
    );
}

#[test]
fn test_claim_lp_staking_rewards_auto_stake() {
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::ClaimLpStakingRewards {
        lp_token_contract: MOCK_LP_TOKEN_CONTRACT.to_owned(),
        auto_stake: Some(true),
    };

    let info = mock_info("sender", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "claim_lp_staking_rewards")
            .add_attribute("action", "stake_astro")
            .add_attribute("amount", "110")
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_ASTROPORT_GENERATOR_CONTRACT.to_owned(),
                funds: vec![],
                msg: to_binary(&GeneratorExecuteMsg::Withdraw {
                    amount: Uint128::zero(),
                    lp_token: Addr::unchecked(MOCK_LP_TOKEN_CONTRACT),
                })
                .unwrap(),
            }))
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_ASTRO_TOKEN.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: MOCK_ASTROPORT_STAKING_CONTRACT.to_string(),
                    amount: 110u128.into(),
                    msg: to_binary(&StakingCw20HookMsg::Enter {}).unwrap(),
                })
                .unwrap(),
                funds: vec![],
            })),
        res
    );
}

#[test]
fn test_claim_lp_staking_rewards_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::ClaimLpStakingRewards {
        lp_token_contract: MOCK_LP_TOKEN_CONTRACT.to_owned(),
        auto_stake: None,
    };

    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_stake_astro() {
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::StakeAstro { amount: None };

    let info = mock_info("sender", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "stake_astro")
            .add_attribute("amount", "10")
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_ASTRO_TOKEN.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: MOCK_ASTROPORT_STAKING_CONTRACT.to_string(),
                    amount: 10u128.into(),
                    msg: to_binary(&StakingCw20HookMsg::Enter {}).unwrap(),
                })
                .unwrap(),
                funds: vec![],
            })),
        res
    );
}

#[test]
fn test_stake_astro_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::StakeAstro { amount: None };

    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_unstake_astro() {
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::UnstakeAstro { amount: None };

    let info = mock_info("sender", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "unstake_astro")
            .add_attribute("amount", "10")
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_XASTRO_TOKEN.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: MOCK_ASTROPORT_STAKING_CONTRACT.to_string(),
                    amount: 10u128.into(),
                    msg: to_binary(&StakingCw20HookMsg::Leave {}).unwrap(),
                })
                .unwrap(),
                funds: vec![],
            })),
        res
    );
}

#[test]
fn test_unstake_astro_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut());

    let msg = ExecuteMsg::UnstakeAstro { amount: None };

    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}
