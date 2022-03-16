use crate::{
    contract::{execute, instantiate},
    state::{Config, Purchase, State, CONFIG, PURCHASES, STATE, UNAVAILABLE_TOKENS},
    testing::mock_querier::{
        mock_dependencies_custom, MOCK_NON_EXISTING_TOKEN, MOCK_PRIMITIVE_CONTRACT,
        MOCK_RATES_CONTRACT, MOCK_RATES_RECIPIENT, MOCK_TOKENS_FOR_SALE, MOCK_TOKEN_CONTRACT,
    },
};
use andromeda_protocol::crowdfund::{ExecuteMsg, InstantiateMsg};
use common::{
    ado_base::{
        modules::{InstantiateType, Module, RATES},
        recipient::Recipient,
    },
    error::ContractError,
};
use cosmwasm_std::{
    coin, coins,
    testing::{mock_env, mock_info},
    Addr, BankMsg, Coin, CosmosMsg, DepsMut, Response, SubMsg, Uint128,
};
use cw0::Expiration;

fn get_rates_messages() -> Vec<SubMsg> {
    let coin = coin(100u128, "uusd");
    vec![
        SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: MOCK_RATES_RECIPIENT.to_owned(),
            amount: vec![Coin {
                // Royalty of 10%
                amount: coin.amount.multiply_ratio(10u128, 100u128),
                denom: coin.denom.clone(),
            }],
        })),
        SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: MOCK_RATES_RECIPIENT.to_owned(),
            amount: vec![Coin {
                // Flat tax of 50
                amount: Uint128::from(50u128),
                denom: coin.denom.clone(),
            }],
        })),
    ]
}

fn init(deps: DepsMut, modules: Option<Vec<Module>>) -> Response {
    let msg = InstantiateMsg {
        token_address: MOCK_TOKEN_CONTRACT.to_owned(),
        modules,
        primitive_address: MOCK_PRIMITIVE_CONTRACT.to_owned(),
    };

    let info = mock_info("owner", &[]);
    instantiate(deps, mock_env(), info, msg).unwrap()
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);

    let modules = vec![Module {
        module_type: RATES.to_owned(),
        instantiate: InstantiateType::Address(MOCK_RATES_CONTRACT.to_owned()),
        is_mutable: false,
    }];

    let res = init(deps.as_mut(), Some(modules));

    assert_eq!(
        Response::new()
            .add_attribute("method", "instantiate")
            .add_attribute("type", "crowdfund"),
        res
    );

    assert_eq!(
        Config {
            token_address: Addr::unchecked(MOCK_TOKEN_CONTRACT),
        },
        CONFIG.load(deps.as_mut().storage).unwrap()
    );
}

#[test]
fn test_start_sale_no_expiration() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    let msg = ExecuteMsg::StartSale {
        expiration: Expiration::Never {},
        price: coin(100, "uusd"),
        min_tokens_sold: Uint128::from(1u128),
        max_amount_per_wallet: None,
        recipient: Recipient::Addr("recipient".to_string()),
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::ExpirationMustNotBeNever {}, res.unwrap_err());
}

#[test]
fn test_start_sale_expiration_in_past() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    let msg = ExecuteMsg::StartSale {
        expiration: Expiration::AtHeight(mock_env().block.height - 1),
        price: coin(100, "uusd"),
        min_tokens_sold: Uint128::from(1u128),
        max_amount_per_wallet: None,
        recipient: Recipient::Addr("recipient".to_string()),
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::ExpirationInPast {}, res.unwrap_err());
}

#[test]
fn test_start_sale_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    let msg = ExecuteMsg::StartSale {
        expiration: Expiration::AtHeight(mock_env().block.height + 1),
        price: coin(100, "uusd"),
        min_tokens_sold: Uint128::from(1u128),
        max_amount_per_wallet: None,
        recipient: Recipient::Addr("recipient".to_string()),
    };

    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_start_sale_max_default() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    let msg = ExecuteMsg::StartSale {
        expiration: Expiration::AtHeight(mock_env().block.height + 1),
        price: coin(100, "uusd"),
        min_tokens_sold: Uint128::from(1u128),
        max_amount_per_wallet: None,
        recipient: Recipient::Addr("recipient".to_string()),
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        Response::new()
            .add_attribute("action", "start_sale")
            .add_attribute("expiration", "expiration height: 12346")
            .add_attribute("price", "100uusd")
            .add_attribute("min_tokens_sold", "1")
            .add_attribute("max_amount_per_wallet", "1"),
        res
    );

    assert_eq!(
        State {
            expiration: Expiration::AtHeight(mock_env().block.height + 1),
            price: coin(100, "uusd"),
            min_tokens_sold: Uint128::from(1u128),
            max_amount_per_wallet: Uint128::from(1u128),
            amount_sold: Uint128::zero(),
            amount_to_send: Uint128::zero(),
            recipient: Recipient::Addr("recipient".to_string()),
        },
        STATE.load(deps.as_ref().storage).unwrap()
    );
}

#[test]
fn test_start_sale_max_modified() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    let msg = ExecuteMsg::StartSale {
        expiration: Expiration::AtHeight(mock_env().block.height + 1),
        price: coin(100, "uusd"),
        min_tokens_sold: Uint128::from(1u128),
        max_amount_per_wallet: Some(Uint128::from(5u128)),
        recipient: Recipient::Addr("recipient".to_string()),
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        Response::new()
            .add_attribute("action", "start_sale")
            .add_attribute("expiration", "expiration height: 12346")
            .add_attribute("price", "100uusd")
            .add_attribute("min_tokens_sold", "1")
            .add_attribute("max_amount_per_wallet", "5"),
        res
    );

    assert_eq!(
        State {
            expiration: Expiration::AtHeight(mock_env().block.height + 1),
            price: coin(100, "uusd"),
            min_tokens_sold: Uint128::from(1u128),
            max_amount_per_wallet: Uint128::from(5u128),
            amount_sold: Uint128::zero(),
            amount_to_send: Uint128::zero(),
            recipient: Recipient::Addr("recipient".to_string()),
        },
        STATE.load(deps.as_ref().storage).unwrap()
    );
}

#[test]
fn test_purchase_sale_not_started() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    let msg = ExecuteMsg::Purchase {
        token_id: "token_id".to_string(),
    };

    let info = mock_info("sender", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::NoOngoingSale {}, res.unwrap_err());
}

#[test]
fn test_purchase_sale_not_ended() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    let msg = ExecuteMsg::Purchase {
        token_id: "token_id".to_string(),
    };

    STATE
        .save(
            deps.as_mut().storage,
            &State {
                expiration: Expiration::AtHeight(mock_env().block.height - 1),
                price: coin(100, "uusd"),
                min_tokens_sold: Uint128::from(1u128),
                max_amount_per_wallet: Uint128::from(5u128),
                amount_sold: Uint128::zero(),
                amount_to_send: Uint128::zero(),
                recipient: Recipient::Addr("recipient".to_string()),
            },
        )
        .unwrap();

    let info = mock_info("sender", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::NoOngoingSale {}, res.unwrap_err());
}

macro_rules! purchase_not_for_sale_tests {
    ($($name:ident: $token_id:expr,)*) => {
    $(
        #[test]
        fn $name() {
            let mut deps = mock_dependencies_custom(&[]);
            init(deps.as_mut(), None);

            let msg = ExecuteMsg::Purchase {
                token_id: $token_id,
            };

            STATE
                .save(
                    deps.as_mut().storage,
                    &State {
                        expiration: Expiration::AtHeight(mock_env().block.height + 1),
                        price: coin(100, "uusd"),
                        min_tokens_sold: Uint128::from(1u128),
                        max_amount_per_wallet: Uint128::from(5u128),
                        amount_sold: Uint128::zero(),
                        amount_to_send: Uint128::zero(),
                        recipient: Recipient::Addr("recipient".to_string()),
                    },
                )
                .unwrap();

            let info = mock_info("sender", &[]);
            let res = execute(deps.as_mut(), mock_env(), info, msg);
            assert_eq!(ContractError::TokenNotForSale {}, res.unwrap_err());
        }
    )*
    }
}

purchase_not_for_sale_tests! {
    test_purchase_existing_token_not_for_sale: ("token_not_for_sale".to_string()),
    test_purchase_not_existing_token_not_for_sale: MOCK_NON_EXISTING_TOKEN.to_string(),
}

#[test]
fn test_purchase_no_funds() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    let msg = ExecuteMsg::Purchase {
        token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
    };

    STATE
        .save(
            deps.as_mut().storage,
            &State {
                expiration: Expiration::AtHeight(mock_env().block.height + 1),
                price: coin(100, "uusd"),
                min_tokens_sold: Uint128::from(1u128),
                max_amount_per_wallet: Uint128::from(5u128),
                amount_sold: Uint128::zero(),
                amount_to_send: Uint128::zero(),
                recipient: Recipient::Addr("recipient".to_string()),
            },
        )
        .unwrap();

    let info = mock_info("sender", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());
}

#[test]
fn test_purchase_wrong_denom() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    let msg = ExecuteMsg::Purchase {
        token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
    };

    STATE
        .save(
            deps.as_mut().storage,
            &State {
                expiration: Expiration::AtHeight(mock_env().block.height + 1),
                price: coin(100, "uusd"),
                min_tokens_sold: Uint128::from(1u128),
                max_amount_per_wallet: Uint128::from(5u128),
                amount_sold: Uint128::zero(),
                amount_to_send: Uint128::zero(),
                recipient: Recipient::Addr("recipient".to_string()),
            },
        )
        .unwrap();

    let info = mock_info("sender", &coins(100, "uluna"));
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());
}

#[test]
fn test_purchase_not_enough_for_price() {
    let mut deps = mock_dependencies_custom(&[]);
    let modules = vec![Module {
        module_type: RATES.to_owned(),
        instantiate: InstantiateType::Address(MOCK_RATES_CONTRACT.to_owned()),
        is_mutable: false,
    }];
    init(deps.as_mut(), Some(modules));

    let msg = ExecuteMsg::Purchase {
        token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
    };

    STATE
        .save(
            deps.as_mut().storage,
            &State {
                expiration: Expiration::AtHeight(mock_env().block.height + 1),
                price: coin(100, "uusd"),
                min_tokens_sold: Uint128::from(1u128),
                max_amount_per_wallet: Uint128::from(5u128),
                amount_sold: Uint128::zero(),
                amount_to_send: Uint128::zero(),
                recipient: Recipient::Addr("recipient".to_string()),
            },
        )
        .unwrap();

    let info = mock_info("sender", &coins(50u128, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());
}

#[test]
fn test_purchase_not_enough_for_tax() {
    let mut deps = mock_dependencies_custom(&[]);
    let modules = vec![Module {
        module_type: RATES.to_owned(),
        instantiate: InstantiateType::Address(MOCK_RATES_CONTRACT.to_owned()),
        is_mutable: false,
    }];
    init(deps.as_mut(), Some(modules));

    let msg = ExecuteMsg::Purchase {
        token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
    };

    STATE
        .save(
            deps.as_mut().storage,
            &State {
                expiration: Expiration::AtHeight(mock_env().block.height + 1),
                price: coin(100, "uusd"),
                min_tokens_sold: Uint128::from(1u128),
                max_amount_per_wallet: Uint128::from(5u128),
                amount_sold: Uint128::zero(),
                amount_to_send: Uint128::zero(),
                recipient: Recipient::Addr("recipient".to_string()),
            },
        )
        .unwrap();

    let info = mock_info("sender", &coins(100u128, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());
}

#[test]
fn test_multiple_purchases() {
    let mut deps = mock_dependencies_custom(&[]);
    let modules = vec![Module {
        module_type: RATES.to_owned(),
        instantiate: InstantiateType::Address(MOCK_RATES_CONTRACT.to_owned()),
        is_mutable: false,
    }];
    init(deps.as_mut(), Some(modules));

    let msg = ExecuteMsg::Purchase {
        token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
    };

    let mut state = State {
        expiration: Expiration::AtHeight(mock_env().block.height + 1),
        price: coin(100, "uusd"),
        min_tokens_sold: Uint128::from(1u128),
        max_amount_per_wallet: Uint128::from(2u128),
        amount_sold: Uint128::zero(),
        amount_to_send: Uint128::zero(),
        recipient: Recipient::Addr("recipient".to_string()),
    };
    STATE.save(deps.as_mut().storage, &state).unwrap();

    let info = mock_info("sender", &coins(150u128, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "purchase")
            .add_attribute("token_id", MOCK_TOKENS_FOR_SALE[0]),
        res
    );

    state.amount_to_send += Uint128::from(90u128);
    state.amount_sold += Uint128::from(1u128);
    assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

    assert!(UNAVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[0]));

    let first_purchase = Purchase {
        token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
        purchaser: "sender".to_string(),
        tax_amount: Uint128::from(50u128),
        msgs: get_rates_messages(),
    };

    assert_eq!(
        vec![first_purchase],
        PURCHASES.load(deps.as_ref().storage, "sender").unwrap()
    )
}
