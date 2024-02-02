use andromeda_non_fungible_tokens::marketplace::{
    Cw721HookMsg, ExecuteMsg, InstantiateMsg, Status,
};
use andromeda_std::{
    ado_base::modules::Module, amp::addresses::AndrAddr, common::encode_binary,
    error::ContractError,
};
use cosmwasm_std::{
    coin, coins,
    testing::{mock_env, mock_info},
    BankMsg, CosmosMsg, Deps, DepsMut, Response, SubMsg, Uint128, WasmMsg,
};
use cw721::{Cw721ExecuteMsg, Cw721ReceiveMsg};

use super::mock_querier::MOCK_KERNEL_CONTRACT;
use crate::{
    contract::{execute, instantiate},
    state::{sale_infos, SaleInfo, TokenSaleState, TOKEN_SALE_STATE},
    testing::mock_querier::{
        mock_dependencies_custom, MOCK_RATES_CONTRACT, MOCK_TOKEN_ADDR, MOCK_TOKEN_OWNER,
        MOCK_UNCLAIMED_TOKEN, RATES,
    },
};

fn start_sale(deps: DepsMut) {
    let hook_msg = Cw721HookMsg::StartSale {
        coin_denom: "uusd".to_string(),
        price: Uint128::new(100),
    };
    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: MOCK_TOKEN_OWNER.to_owned(),
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        msg: encode_binary(&hook_msg).unwrap(),
    });
    let env = mock_env();

    let info = mock_info(MOCK_TOKEN_ADDR, &[]);
    let _res = execute(deps, env, info, msg).unwrap();
}

fn init(deps: DepsMut, modules: Option<Vec<Module>>) -> Response {
    let msg = InstantiateMsg {
        owner: None,
        modules,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
    };

    let info = mock_info("owner", &[]);
    instantiate(deps, mock_env(), info, msg).unwrap()
}

fn assert_sale_created(deps: Deps) {
    assert_eq!(
        TokenSaleState {
            coin_denom: "uusd".to_string(),
            sale_id: 1u128.into(),
            owner: MOCK_TOKEN_OWNER.to_string(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_owned(),
            status: Status::Open,
            price: Uint128::new(100)
        },
        TOKEN_SALE_STATE.load(deps.storage, 1u128).unwrap()
    );

    assert_eq!(
        SaleInfo {
            sale_ids: vec![Uint128::from(1u128)],
            token_address: MOCK_TOKEN_ADDR.to_owned(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        },
        sale_infos()
            .load(
                deps.storage,
                &(MOCK_UNCLAIMED_TOKEN.to_owned() + MOCK_TOKEN_ADDR)
            )
            .unwrap()
    );
}

#[test]
fn test_sale_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);
    let res = init(deps.as_mut(), None);
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_execute_buy_non_existing_sale() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut(), None);
    let env = mock_env();
    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_string(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };
    let info = mock_info("buyer", &coins(100, "uusd"));
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::SaleDoesNotExist {}, res.unwrap_err());
}

#[test]
fn execute_buy_sale_not_open_already_bought() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref());

    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info("sender", &coins(100, "uusd".to_string()));
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info("sender", &coins(100, "uusd".to_string()));
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::SaleNotOpen {})
}

#[test]
fn execute_buy_sale_not_open_cancelled() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let _res = init(deps.as_mut(), None);

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref());

    let msg = ExecuteMsg::CancelSale {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info(MOCK_TOKEN_OWNER, &[]);
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };
    let info = mock_info("sender", &coins(100, "uusd".to_string()));
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::SaleNotOpen {})
}

#[test]
fn execute_buy_token_owner_cannot_buy() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let _res = init(deps.as_mut(), None);

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref());

    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info(MOCK_TOKEN_OWNER, &coins(100, "uusd".to_string()));
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::TokenOwnerCannotBuy {}, res.unwrap_err());
}

// #[test]
// fn execute_buy_whitelist() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let env = mock_env();
//     let info = mock_info("owner", &[]);
//     let msg = InstantiateMsg {
//     let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

//     start_sale(deps.as_mut(), Some(vec![Addr::unchecked("sender")]));
//     assert_sale_created(deps.as_ref(), Some(vec![Addr::unchecked("sender")]));

//     let msg = ExecuteMsg::Buy {
//         token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
//         token_address: MOCK_TOKEN_ADDR.to_string(),
//     };

//     let info = mock_info("not_sender", &coins(100, "uusd".to_string()));
//     let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
//     assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

//     let info = mock_info("sender", &coins(100, "uusd".to_string()));
//     let _res = execute(deps.as_mut(), env, info, msg).unwrap();
// }

#[test]
fn execute_buy_invalid_coins_sent() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let _res = init(deps.as_mut(), None);

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref());

    let error = ContractError::InvalidFunds {
        msg: "Sales ensure! exactly one coin to be sent.".to_string(),
    };
    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_string(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    // No coins sent
    let info = mock_info("sender", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_eq!(error, res.unwrap_err());

    // Multiple coins sent
    let info = mock_info("sender", &[coin(100, "uusd"), coin(100, "uluna")]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_eq!(error, res.unwrap_err());

    // Invalid denom sent
    let info = mock_info("sender", &[coin(100, "uluna")]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_eq!(
        ContractError::InvalidFunds {
            msg: "No uusd assets are provided to sale".to_string(),
        },
        res.unwrap_err()
    );

    // Correct denom but empty
    let info = mock_info("sender", &[coin(0, "uusd")]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert!(matches!(res, ContractError::InvalidFunds { .. }));
}

#[test]
fn execute_buy_works() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let _res = init(deps.as_mut(), None);

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref());

    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info("someone", &coins(100, "uusd".to_string()));
    let _res = execute(deps.as_mut(), env, info, msg).unwrap();
}

#[test]
fn execute_update_sale_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let _res = init(deps.as_mut(), None);

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref());

    let msg = ExecuteMsg::UpdateSale {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
        price: Uint128::new(11),
        coin_denom: "juno".to_string(),
    };

    let info = mock_info("someone", &[]);
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {})
}

#[test]
fn execute_update_sale_invalid_price() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let _res = init(deps.as_mut(), None);

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref());

    let msg = ExecuteMsg::UpdateSale {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
        price: Uint128::zero(),
        coin_denom: "juno".to_string(),
    };

    let info = mock_info("owner", &[]);
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::InvalidZeroAmount {})
}

#[test]
fn execute_start_sale_invalid_price() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut(), None);

    let hook_msg = Cw721HookMsg::StartSale {
        coin_denom: "uusd".to_string(),
        price: Uint128::zero(),
    };
    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: MOCK_TOKEN_OWNER.to_owned(),
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        msg: encode_binary(&hook_msg).unwrap(),
    });
    let env = mock_env();

    let info = mock_info(MOCK_TOKEN_ADDR, &[]);
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::InvalidZeroAmount {})
}

#[test]
fn execute_buy_with_tax_and_royalty_insufficient_funds() {
    let mut deps = mock_dependencies_custom(&[]);
    let modules = vec![Module {
        name: Some(RATES.to_owned()),
        address: AndrAddr::from_string(MOCK_RATES_CONTRACT.to_owned()),
        is_mutable: false,
    }];
    let _res = init(deps.as_mut(), Some(modules));

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref());

    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info("someone", &coins(100, "uusd".to_string()));
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert!(matches!(err, ContractError::InvalidFunds { .. }));
}

#[test]
fn execute_buy_with_tax_and_royalty_too_many_funds() {
    let mut deps = mock_dependencies_custom(&[]);
    let modules = vec![Module {
        name: Some(RATES.to_owned()),
        address: AndrAddr::from_string(MOCK_RATES_CONTRACT.to_owned()),
        is_mutable: false,
    }];
    let _res = init(deps.as_mut(), Some(modules));

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref());

    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info("someone", &coins(200, "uusd".to_string()));
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert!(matches!(err, ContractError::InvalidFunds { .. }));
}

#[test]
fn execute_buy_with_tax_and_royalty_works() {
    let mut deps = mock_dependencies_custom(&[]);
    let modules = vec![Module {
        name: Some(RATES.to_owned()),
        address: AndrAddr::from_string(MOCK_RATES_CONTRACT.to_owned()),
        is_mutable: false,
    }];
    let _res = init(deps.as_mut(), Some(modules));

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref());

    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info("someone", &coins(150, "uusd".to_string()));
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let expected: Vec<SubMsg<_>> = vec![
        SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "royalty_recipient".to_string(),
            amount: vec![coin(10, "uusd")],
        })),
        SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "tax_recipient".to_string(),
            amount: vec![coin(50, "uusd")],
        })),
        SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "owner".to_string(),
            amount: vec![coin(90, "uusd")],
        })),
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_TOKEN_ADDR.to_string(),
            msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: info.sender.to_string(),
                token_id: MOCK_UNCLAIMED_TOKEN.to_string(),
            })
            .unwrap(),
            funds: vec![],
        })),
    ];
    assert_eq!(res.messages, expected)
}
