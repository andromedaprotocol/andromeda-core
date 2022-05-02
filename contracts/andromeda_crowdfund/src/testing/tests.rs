use crate::{
    contract::{execute, instantiate, query, MAX_MINT_LIMIT},
    state::{
        Config, Purchase, State, AVAILABLE_TOKENS, CONFIG, NUMBER_OF_TOKENS_AVAILABLE, PURCHASES,
        SALE_CONDUCTED, STATE,
    },
    testing::mock_querier::{
        mock_dependencies_custom, MOCK_ADDRESSLIST_CONTRACT, MOCK_CONDITIONS_MET_CONTRACT,
        MOCK_CONDITIONS_NOT_MET_CONTRACT, MOCK_MISSION_CONTRACT, MOCK_RATES_CONTRACT,
        MOCK_ROYALTY_RECIPIENT, MOCK_TAX_RECIPIENT, MOCK_TOKENS_FOR_SALE, MOCK_TOKEN_CONTRACT,
    },
};
use ado_base::ADOContract;
use andromeda_protocol::{
    crowdfund::{CrowdfundMintMsg, ExecuteMsg, InstantiateMsg, QueryMsg},
    cw721::{ExecuteMsg as Cw721ExecuteMsg, MintMsg, TokenExtension},
};
use common::{
    ado_base::{
        modules::{Module, ADDRESS_LIST, RATES},
        recipient::Recipient,
        AndromedaMsg,
    },
    encode_binary,
    error::ContractError,
    mission::AndrAddress,
};
use cosmwasm_std::{
    coin, coins, from_binary,
    testing::{mock_env, mock_info},
    Addr, BankMsg, Coin, CosmosMsg, DepsMut, Response, StdError, SubMsg, Uint128, WasmMsg,
};
use cw0::Expiration;

fn get_purchase(token_id: impl Into<String>, purchaser: impl Into<String>) -> Purchase {
    Purchase {
        token_id: token_id.into(),
        purchaser: purchaser.into(),
        tax_amount: Uint128::from(50u128),
        msgs: get_rates_messages(),
    }
}

fn get_rates_messages() -> Vec<SubMsg> {
    let coin = coin(100u128, "uusd");
    vec![
        SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: MOCK_ROYALTY_RECIPIENT.to_owned(),
            amount: vec![Coin {
                // Royalty of 10%
                amount: coin.amount.multiply_ratio(10u128, 100u128),
                denom: coin.denom.clone(),
            }],
        })),
        SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: MOCK_TAX_RECIPIENT.to_owned(),
            amount: vec![Coin {
                // Flat tax of 50
                amount: Uint128::from(50u128),
                denom: coin.denom,
            }],
        })),
    ]
}

fn get_burn_message(token_id: impl Into<String>) -> CosmosMsg {
    CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: MOCK_TOKEN_CONTRACT.to_owned(),
        funds: vec![],
        msg: encode_binary(&Cw721ExecuteMsg::Burn {
            token_id: token_id.into(),
        })
        .unwrap(),
    })
}

fn get_transfer_message(token_id: impl Into<String>, recipient: impl Into<String>) -> CosmosMsg {
    CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: MOCK_TOKEN_CONTRACT.to_owned(),
        msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
            recipient: recipient.into(),
            token_id: token_id.into(),
        })
        .unwrap(),
        funds: vec![],
    })
}

fn init(deps: DepsMut, modules: Option<Vec<Module>>) -> Response {
    let msg = InstantiateMsg {
        token_address: AndrAddress {
            identifier: MOCK_TOKEN_CONTRACT.to_owned(),
        },
        modules,
        can_mint_after_sale: true,
    };

    let info = mock_info("owner", &[]);
    instantiate(deps, mock_env(), info, msg).unwrap()
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);

    let modules = vec![Module {
        module_type: RATES.to_owned(),
        address: AndrAddress {
            identifier: MOCK_RATES_CONTRACT.to_owned(),
        },
        is_mutable: false,
    }];

    let res = init(deps.as_mut(), Some(modules));

    assert_eq!(
        Response::new()
            .add_attribute("action", "register_module")
            .add_attribute("module_idx", "1")
            .add_attribute("method", "instantiate")
            .add_attribute("type", "crowdfund"),
        res
    );

    assert_eq!(
        Config {
            token_address: AndrAddress {
                identifier: MOCK_TOKEN_CONTRACT.to_owned()
            },
            can_mint_after_sale: true
        },
        CONFIG.load(deps.as_mut().storage).unwrap()
    );

    assert!(!SALE_CONDUCTED.load(deps.as_mut().storage).unwrap());
}

#[test]
fn test_mint_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    let msg = ExecuteMsg::Mint(vec![CrowdfundMintMsg {
        token_id: "token_id".to_string(),
        owner: None,
        token_uri: None,
        extension: TokenExtension {
            name: "name".to_string(),
            publisher: "publisher".to_string(),
            description: None,
            transfer_agreement: None,
            metadata: None,
            archived: false,
        },
    }]);
    let info = mock_info("not_owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_mint_owner_not_crowdfund() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    let msg = ExecuteMsg::Mint(vec![CrowdfundMintMsg {
        token_id: "token_id".to_string(),
        owner: Some("not_crowdfund".to_string()),
        token_uri: None,
        extension: TokenExtension {
            name: "name".to_string(),
            publisher: "publisher".to_string(),
            description: None,
            transfer_agreement: None,
            metadata: None,
            archived: false,
        },
    }]);
    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Since token was minted to owner that is not the contract, it is not available for sale.
    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, "token_id"));
}

#[test]
fn test_mint_sale_started() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    let msg = ExecuteMsg::StartSale {
        expiration: Expiration::AtHeight(mock_env().block.height + 1),
        price: coin(100, "uusd"),
        min_tokens_sold: Uint128::from(1u128),
        max_amount_per_wallet: Some(5),
        recipient: Recipient::Addr("recipient".to_string()),
    };

    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = mint(deps.as_mut(), "token_id");

    assert_eq!(ContractError::SaleStarted {}, res.unwrap_err());
}

#[test]
fn test_mint_sale_conducted_cant_mint_after_sale() {
    let mut deps = mock_dependencies_custom(&[]);
    let msg = InstantiateMsg {
        token_address: AndrAddress {
            identifier: MOCK_TOKEN_CONTRACT.to_owned(),
        },
        modules: None,
        can_mint_after_sale: false,
    };

    let info = mock_info("owner", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    SALE_CONDUCTED.save(deps.as_mut().storage, &true).unwrap();

    let res = mint(deps.as_mut(), "token_id");

    assert_eq!(
        ContractError::CannotMintAfterSaleConducted {},
        res.unwrap_err()
    );
}

#[test]
fn test_mint_sale_conducted_can_mint_after_sale() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    SALE_CONDUCTED.save(deps.as_mut().storage, &true).unwrap();

    let _res = mint(deps.as_mut(), "token_id").unwrap();

    assert!(AVAILABLE_TOKENS.has(deps.as_ref().storage, "token_id"));
}

#[test]
fn test_mint_successful() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    let res = mint(deps.as_mut(), "token_id").unwrap();

    let mint_msg = Box::new(MintMsg {
        token_id: "token_id".to_string(),
        owner: mock_env().contract.address.to_string(),
        token_uri: None,
        extension: TokenExtension {
            name: "name".to_string(),
            publisher: "publisher".to_string(),
            description: None,
            transfer_agreement: None,
            metadata: None,
            archived: false,
        },
    });

    assert_eq!(
        Response::new()
            .add_attribute("action", "mint")
            .add_message(WasmMsg::Execute {
                contract_addr: MOCK_TOKEN_CONTRACT.to_owned(),
                msg: encode_binary(&Cw721ExecuteMsg::Mint(mint_msg)).unwrap(),
                funds: vec![],
            }),
        res
    );

    assert!(AVAILABLE_TOKENS.has(deps.as_ref().storage, "token_id"));
}

#[test]
fn test_mint_multiple_successful() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    let mint_msgs = vec![
        CrowdfundMintMsg {
            token_id: "token_id1".to_string(),
            owner: None,
            token_uri: None,
            extension: TokenExtension {
                name: "name1".to_string(),
                publisher: "publisher".to_string(),
                description: None,
                transfer_agreement: None,
                metadata: None,
                archived: false,
            },
        },
        CrowdfundMintMsg {
            token_id: "token_id2".to_string(),
            owner: None,
            token_uri: None,
            extension: TokenExtension {
                name: "name2".to_string(),
                publisher: "publisher".to_string(),
                description: None,
                transfer_agreement: None,
                metadata: None,
                archived: false,
            },
        },
    ];

    let msg = ExecuteMsg::Mint(mint_msgs);
    let res = execute(deps.as_mut(), mock_env(), mock_info("owner", &[]), msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "mint")
            .add_attribute("action", "mint")
            .add_message(WasmMsg::Execute {
                contract_addr: MOCK_TOKEN_CONTRACT.to_owned(),
                msg: encode_binary(&Cw721ExecuteMsg::Mint(Box::new(MintMsg {
                    token_id: "token_id1".to_string(),
                    owner: mock_env().contract.address.to_string(),
                    token_uri: None,
                    extension: TokenExtension {
                        name: "name1".to_string(),
                        publisher: "publisher".to_string(),
                        description: None,
                        transfer_agreement: None,
                        metadata: None,
                        archived: false,
                    },
                },)))
                .unwrap(),
                funds: vec![],
            })
            .add_message(WasmMsg::Execute {
                contract_addr: MOCK_TOKEN_CONTRACT.to_owned(),
                msg: encode_binary(&Cw721ExecuteMsg::Mint(Box::new(MintMsg {
                    token_id: "token_id2".to_string(),
                    owner: mock_env().contract.address.to_string(),
                    token_uri: None,
                    extension: TokenExtension {
                        name: "name2".to_string(),
                        publisher: "publisher".to_string(),
                        description: None,
                        transfer_agreement: None,
                        metadata: None,
                        archived: false,
                    },
                },)))
                .unwrap(),
                funds: vec![],
            }),
        res
    );

    assert!(AVAILABLE_TOKENS.has(deps.as_ref().storage, "token_id1"));
    assert!(AVAILABLE_TOKENS.has(deps.as_ref().storage, "token_id2"));

    assert_eq!(
        NUMBER_OF_TOKENS_AVAILABLE
            .load(deps.as_ref().storage)
            .unwrap(),
        Uint128::new(2)
    );
}

#[test]
fn test_mint_multiple_exceeds_limit() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    let mint_msg = CrowdfundMintMsg {
        token_id: "token_id1".to_string(),
        owner: None,
        token_uri: None,
        extension: TokenExtension {
            name: "name1".to_string(),
            publisher: "publisher".to_string(),
            description: None,
            transfer_agreement: None,
            metadata: None,
            archived: false,
        },
    };

    let mut mint_msgs: Vec<CrowdfundMintMsg> = vec![];

    for _ in 0..MAX_MINT_LIMIT + 1 {
        mint_msgs.push(mint_msg.clone());
    }

    let msg = ExecuteMsg::Mint(mint_msgs.clone());
    let res = execute(deps.as_mut(), mock_env(), mock_info("owner", &[]), msg);

    assert_eq!(
        ContractError::TooManyMintMessages {
            limit: MAX_MINT_LIMIT
        },
        res.unwrap_err()
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
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
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
            max_amount_per_wallet: 1,
            amount_sold: Uint128::zero(),
            amount_to_send: Uint128::zero(),
            amount_transferred: Uint128::zero(),
            recipient: Recipient::Addr("recipient".to_string()),
        },
        STATE.load(deps.as_ref().storage).unwrap()
    );

    assert!(SALE_CONDUCTED.load(deps.as_ref().storage).unwrap());

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::SaleStarted {}, res.unwrap_err());
}

#[test]
fn test_start_sale_max_modified() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    let msg = ExecuteMsg::StartSale {
        expiration: Expiration::AtHeight(mock_env().block.height + 1),
        price: coin(100, "uusd"),
        min_tokens_sold: Uint128::from(1u128),
        max_amount_per_wallet: Some(5),
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
            max_amount_per_wallet: 5,
            amount_sold: Uint128::zero(),
            amount_to_send: Uint128::zero(),
            amount_transferred: Uint128::zero(),
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
        number_of_tokens: None,
    };

    let info = mock_info("sender", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    assert_eq!(ContractError::NoOngoingSale {}, res.unwrap_err());

    let msg = ExecuteMsg::PurchaseByTokenId {
        token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::NoOngoingSale {}, res.unwrap_err());
}

#[test]
fn test_purchase_sale_not_ended() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    STATE
        .save(
            deps.as_mut().storage,
            &State {
                expiration: Expiration::AtHeight(mock_env().block.height - 1),
                price: coin(100, "uusd"),
                min_tokens_sold: Uint128::from(1u128),
                max_amount_per_wallet: 5,
                amount_sold: Uint128::zero(),
                amount_to_send: Uint128::zero(),
                amount_transferred: Uint128::zero(),
                recipient: Recipient::Addr("recipient".to_string()),
            },
        )
        .unwrap();

    let info = mock_info("sender", &[]);

    let msg = ExecuteMsg::Purchase {
        number_of_tokens: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    assert_eq!(ContractError::NoOngoingSale {}, res.unwrap_err());

    let msg = ExecuteMsg::PurchaseByTokenId {
        token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::NoOngoingSale {}, res.unwrap_err());
}

#[test]
fn test_purchase_no_funds() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[0]).unwrap();

    STATE
        .save(
            deps.as_mut().storage,
            &State {
                expiration: Expiration::AtHeight(mock_env().block.height + 1),
                price: coin(100, "uusd"),
                min_tokens_sold: Uint128::from(1u128),
                max_amount_per_wallet: 5,
                amount_sold: Uint128::zero(),
                amount_to_send: Uint128::zero(),
                amount_transferred: Uint128::zero(),
                recipient: Recipient::Addr("recipient".to_string()),
            },
        )
        .unwrap();

    let info = mock_info("sender", &[]);

    let msg = ExecuteMsg::Purchase {
        number_of_tokens: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());

    let msg = ExecuteMsg::PurchaseByTokenId {
        token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());
}

#[test]
fn test_purchase_wrong_denom() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[0]).unwrap();

    STATE
        .save(
            deps.as_mut().storage,
            &State {
                expiration: Expiration::AtHeight(mock_env().block.height + 1),
                price: coin(100, "uusd"),
                min_tokens_sold: Uint128::from(1u128),
                max_amount_per_wallet: 5,
                amount_sold: Uint128::zero(),
                amount_to_send: Uint128::zero(),
                amount_transferred: Uint128::zero(),
                recipient: Recipient::Addr("recipient".to_string()),
            },
        )
        .unwrap();

    let info = mock_info("sender", &coins(100, "uluna"));

    let msg = ExecuteMsg::Purchase {
        number_of_tokens: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());

    let msg = ExecuteMsg::PurchaseByTokenId {
        token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());
}

#[test]
fn test_purchase_not_enough_for_price() {
    let mut deps = mock_dependencies_custom(&[]);
    let modules = vec![Module {
        module_type: RATES.to_owned(),
        address: AndrAddress {
            identifier: MOCK_RATES_CONTRACT.to_owned(),
        },
        is_mutable: false,
    }];
    init(deps.as_mut(), Some(modules));

    mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[0]).unwrap();

    STATE
        .save(
            deps.as_mut().storage,
            &State {
                expiration: Expiration::AtHeight(mock_env().block.height + 1),
                price: coin(100, "uusd"),
                min_tokens_sold: Uint128::from(1u128),
                max_amount_per_wallet: 5,
                amount_sold: Uint128::zero(),
                amount_to_send: Uint128::zero(),
                amount_transferred: Uint128::zero(),
                recipient: Recipient::Addr("recipient".to_string()),
            },
        )
        .unwrap();

    let info = mock_info("sender", &coins(50u128, "uusd"));

    let msg = ExecuteMsg::Purchase {
        number_of_tokens: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());

    let msg = ExecuteMsg::PurchaseByTokenId {
        token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());
}

#[test]
fn test_purchase_not_enough_for_tax() {
    let mut deps = mock_dependencies_custom(&[]);
    let modules = vec![Module {
        module_type: RATES.to_owned(),
        address: AndrAddress {
            identifier: MOCK_RATES_CONTRACT.to_owned(),
        },
        is_mutable: false,
    }];
    init(deps.as_mut(), Some(modules));

    mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[0]).unwrap();

    NUMBER_OF_TOKENS_AVAILABLE
        .save(deps.as_mut().storage, &Uint128::new(1))
        .unwrap();

    STATE
        .save(
            deps.as_mut().storage,
            &State {
                expiration: Expiration::AtHeight(mock_env().block.height + 1),
                price: coin(100, "uusd"),
                min_tokens_sold: Uint128::from(1u128),
                max_amount_per_wallet: 5,
                amount_sold: Uint128::zero(),
                amount_to_send: Uint128::zero(),
                amount_transferred: Uint128::zero(),
                recipient: Recipient::Addr("recipient".to_string()),
            },
        )
        .unwrap();

    let info = mock_info("sender", &coins(100u128, "uusd"));

    let msg = ExecuteMsg::Purchase {
        number_of_tokens: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());

    // Reset the state since state does not roll back on failure in tests like it does in prod.
    AVAILABLE_TOKENS
        .save(deps.as_mut().storage, MOCK_TOKENS_FOR_SALE[0], &true)
        .unwrap();
    NUMBER_OF_TOKENS_AVAILABLE
        .save(deps.as_mut().storage, &Uint128::new(1))
        .unwrap();

    let msg = ExecuteMsg::PurchaseByTokenId {
        token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());
}

#[test]
fn test_purchase_by_token_id_not_available() {
    let mut deps = mock_dependencies_custom(&[]);
    let modules = vec![Module {
        module_type: RATES.to_owned(),
        address: AndrAddress {
            identifier: MOCK_RATES_CONTRACT.to_owned(),
        },
        is_mutable: false,
    }];
    init(deps.as_mut(), Some(modules));

    mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[0]).unwrap();

    STATE
        .save(
            deps.as_mut().storage,
            &State {
                expiration: Expiration::AtHeight(mock_env().block.height + 1),
                price: coin(100, "uusd"),
                min_tokens_sold: Uint128::from(1u128),
                max_amount_per_wallet: 5,
                amount_sold: Uint128::zero(),
                amount_to_send: Uint128::zero(),
                amount_transferred: Uint128::zero(),
                recipient: Recipient::Addr("recipient".to_string()),
            },
        )
        .unwrap();

    let info = mock_info("sender", &coins(150, "uusd"));

    let msg = ExecuteMsg::PurchaseByTokenId {
        token_id: MOCK_TOKENS_FOR_SALE[1].to_owned(),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::TokenNotAvailable {}, res.unwrap_err());
}

#[test]
fn test_purchase_by_token_id() {
    let mut deps = mock_dependencies_custom(&[]);
    let modules = vec![Module {
        module_type: RATES.to_owned(),
        address: AndrAddress {
            identifier: MOCK_RATES_CONTRACT.to_owned(),
        },
        is_mutable: false,
    }];
    init(deps.as_mut(), Some(modules));

    mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[0]).unwrap();
    mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[1]).unwrap();

    let mut state = State {
        expiration: Expiration::AtHeight(mock_env().block.height + 1),
        price: coin(100, "uusd"),
        min_tokens_sold: Uint128::from(1u128),
        max_amount_per_wallet: 1,
        amount_sold: Uint128::zero(),
        amount_to_send: Uint128::zero(),
        amount_transferred: Uint128::zero(),
        recipient: Recipient::Addr("recipient".to_string()),
    };

    STATE.save(deps.as_mut().storage, &state).unwrap();

    let info = mock_info("sender", &coins(150, "uusd"));

    // Purchase a token.
    let msg = ExecuteMsg::PurchaseByTokenId {
        token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        Response::new()
            .add_attribute("action", "purchase")
            .add_attribute("token_id", MOCK_TOKENS_FOR_SALE[0]),
        res
    );

    state.amount_to_send += Uint128::from(90u128);
    state.amount_sold += Uint128::from(1u128);
    assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[0]));
    assert_eq!(
        NUMBER_OF_TOKENS_AVAILABLE
            .load(deps.as_ref().storage)
            .unwrap(),
        Uint128::new(1)
    );

    // Purchase a second one.
    let msg = ExecuteMsg::PurchaseByTokenId {
        token_id: MOCK_TOKENS_FOR_SALE[1].to_owned(),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::PurchaseLimitReached {}, res.unwrap_err());
}

#[test]
fn test_multiple_purchases() {
    let mut deps = mock_dependencies_custom(&[]);
    let modules = vec![Module {
        module_type: RATES.to_owned(),
        address: AndrAddress {
            identifier: MOCK_RATES_CONTRACT.to_owned(),
        },
        is_mutable: false,
    }];
    init(deps.as_mut(), Some(modules));

    // Mint four tokens.
    mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[0]).unwrap();
    mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[1]).unwrap();
    mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[2]).unwrap();
    mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[3]).unwrap();

    // Query available tokens.
    let msg = QueryMsg::AvailableTokens {
        start_after: None,
        limit: None,
    };
    let res: Vec<String> = from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(
        vec![
            MOCK_TOKENS_FOR_SALE[0],
            MOCK_TOKENS_FOR_SALE[1],
            MOCK_TOKENS_FOR_SALE[2],
            MOCK_TOKENS_FOR_SALE[3]
        ],
        res
    );

    // Query if individual token is available
    let msg = QueryMsg::IsTokenAvailable {
        id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
    };
    let res: bool = from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert!(res);

    // Query if another token is available
    let msg = QueryMsg::IsTokenAvailable {
        id: MOCK_TOKENS_FOR_SALE[4].to_owned(),
    };
    let res: bool = from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert!(!res);

    // Purchase 2 tokens
    let msg = ExecuteMsg::Purchase {
        number_of_tokens: Some(2),
    };

    let mut state = State {
        expiration: Expiration::AtHeight(mock_env().block.height + 1),
        price: coin(100, "uusd"),
        min_tokens_sold: Uint128::from(1u128),
        max_amount_per_wallet: 3,
        amount_sold: Uint128::zero(),
        amount_to_send: Uint128::zero(),
        amount_transferred: Uint128::zero(),
        recipient: Recipient::Addr("recipient".to_string()),
    };
    STATE.save(deps.as_mut().storage, &state).unwrap();

    let info = mock_info("sender", &coins(300u128, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "purchase")
            .add_attribute("number_of_tokens_wanted", "2")
            .add_attribute("number_of_tokens_purchased", "2"),
        res
    );

    state.amount_to_send += Uint128::from(180u128);
    state.amount_sold += Uint128::from(2u128);
    assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[0]));
    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[1]));

    assert_eq!(
        vec![
            get_purchase(MOCK_TOKENS_FOR_SALE[0], "sender"),
            get_purchase(MOCK_TOKENS_FOR_SALE[1], "sender")
        ],
        PURCHASES.load(deps.as_ref().storage, "sender").unwrap()
    );

    // Purchase max number of tokens.
    let msg = ExecuteMsg::Purchase {
        number_of_tokens: None,
    };

    let info = mock_info("sender", &coins(300u128, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "sender".to_string(),
                // Refund sent back as they only were able to mint one.
                amount: coins(150, "uusd")
            })
            .add_attribute("action", "purchase")
            .add_attribute("number_of_tokens_wanted", "1")
            .add_attribute("number_of_tokens_purchased", "1"),
        res
    );

    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[2]));
    state.amount_to_send += Uint128::from(90u128);
    state.amount_sold += Uint128::from(1u128);
    assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

    assert_eq!(
        vec![
            get_purchase(MOCK_TOKENS_FOR_SALE[0], "sender"),
            get_purchase(MOCK_TOKENS_FOR_SALE[1], "sender"),
            get_purchase(MOCK_TOKENS_FOR_SALE[2], "sender")
        ],
        PURCHASES.load(deps.as_ref().storage, "sender").unwrap()
    );

    // Try to purchase an additional token when limit has already been reached.
    let msg = ExecuteMsg::Purchase {
        number_of_tokens: Some(1),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::PurchaseLimitReached {}, res.unwrap_err());

    // User 2 tries to purchase 2 but only 1 is left.
    let msg = ExecuteMsg::Purchase {
        number_of_tokens: Some(2),
    };

    let info = mock_info("user2", &coins(300, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "user2".to_string(),
                // Refund sent back as they only were able to mint one.
                amount: coins(150, "uusd")
            })
            .add_attribute("action", "purchase")
            .add_attribute("number_of_tokens_wanted", "2")
            .add_attribute("number_of_tokens_purchased", "1"),
        res
    );

    assert_eq!(
        vec![get_purchase(MOCK_TOKENS_FOR_SALE[3], "user2"),],
        PURCHASES.load(deps.as_ref().storage, "user2").unwrap()
    );
    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[3]));
    state.amount_to_send += Uint128::from(90u128);
    state.amount_sold += Uint128::from(1u128);
    assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

    assert_eq!(
        NUMBER_OF_TOKENS_AVAILABLE
            .load(deps.as_ref().storage)
            .unwrap(),
        Uint128::zero()
    );

    // User 2 tries to purchase again.
    let msg = ExecuteMsg::Purchase {
        number_of_tokens: None,
    };

    let info = mock_info("user2", &coins(150, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::AllTokensPurchased {}, res.unwrap_err());
}

#[test]
fn test_purchase_more_than_allowed_per_wallet() {
    let mut deps = mock_dependencies_custom(&[]);
    let modules = vec![Module {
        module_type: RATES.to_owned(),
        address: AndrAddress {
            identifier: MOCK_RATES_CONTRACT.to_owned(),
        },
        is_mutable: false,
    }];
    init(deps.as_mut(), Some(modules));

    // Mint four tokens.
    mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[0]).unwrap();
    mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[1]).unwrap();
    mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[2]).unwrap();
    mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[3]).unwrap();

    // Try to purchase 4
    let msg = ExecuteMsg::Purchase {
        number_of_tokens: Some(4),
    };

    let state = State {
        expiration: Expiration::AtHeight(mock_env().block.height + 1),
        price: coin(100, "uusd"),
        min_tokens_sold: Uint128::from(1u128),
        max_amount_per_wallet: 3,
        amount_sold: Uint128::zero(),
        amount_to_send: Uint128::zero(),
        amount_transferred: Uint128::zero(),
        recipient: Recipient::Addr("recipient".to_string()),
    };
    STATE.save(deps.as_mut().storage, &state).unwrap();

    let info = mock_info("sender", &coins(600, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "sender".to_string(),
                amount: coins(150, "uusd")
            })
            .add_attribute("action", "purchase")
            // Number got truncated to 3 which is the max possible.
            .add_attribute("number_of_tokens_wanted", "3")
            .add_attribute("number_of_tokens_purchased", "3"),
        res
    );
}

#[test]
fn test_end_sale_not_expired() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    let state = State {
        expiration: Expiration::AtHeight(mock_env().block.height + 1),
        price: coin(100, "uusd"),
        min_tokens_sold: Uint128::from(1u128),
        max_amount_per_wallet: 2,
        amount_sold: Uint128::zero(),
        amount_to_send: Uint128::zero(),
        amount_transferred: Uint128::zero(),
        recipient: Recipient::Addr("recipient".to_string()),
    };
    STATE.save(deps.as_mut().storage, &state).unwrap();
    NUMBER_OF_TOKENS_AVAILABLE
        .save(deps.as_mut().storage, &Uint128::new(1))
        .unwrap();

    let msg = ExecuteMsg::EndSale { limit: None };
    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::SaleNotEnded {}, res.unwrap_err());
}

fn mint(deps: DepsMut, token_id: impl Into<String>) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::Mint(vec![CrowdfundMintMsg {
        token_id: token_id.into(),
        owner: None,
        token_uri: None,
        extension: TokenExtension {
            name: "name".to_string(),
            publisher: "publisher".to_string(),
            description: None,
            transfer_agreement: None,
            metadata: None,
            archived: false,
        },
    }]);
    execute(deps, mock_env(), mock_info("owner", &[]), msg)
}

#[test]
fn test_integration_conditions_not_met() {
    let mut deps = mock_dependencies_custom(&[]);
    let modules = vec![Module {
        module_type: RATES.to_owned(),
        address: AndrAddress {
            identifier: MOCK_RATES_CONTRACT.to_owned(),
        },
        is_mutable: false,
    }];
    init(deps.as_mut(), Some(modules));

    // Mint all tokens.
    for &token_id in MOCK_TOKENS_FOR_SALE {
        let _res = mint(deps.as_mut(), token_id).unwrap();
        assert!(AVAILABLE_TOKENS.has(deps.as_ref().storage, token_id));
    }

    assert_eq!(
        NUMBER_OF_TOKENS_AVAILABLE
            .load(deps.as_ref().storage)
            .unwrap(),
        Uint128::new(7)
    );

    let msg = ExecuteMsg::StartSale {
        expiration: Expiration::AtHeight(mock_env().block.height + 1),
        price: coin(100, "uusd"),
        min_tokens_sold: Uint128::from(5u128),
        max_amount_per_wallet: Some(2),
        recipient: Recipient::Addr("recipient".to_string()),
    };

    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Can't mint once sale started.
    let res = mint(deps.as_mut(), "token_id");
    assert_eq!(ContractError::SaleStarted {}, res.unwrap_err());

    let msg = ExecuteMsg::Purchase {
        number_of_tokens: Some(1),
    };
    let info = mock_info("A", &coins(150, "uusd"));
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::Purchase {
        number_of_tokens: Some(1),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::Purchase {
        number_of_tokens: Some(1),
    };
    let info = mock_info("B", &coins(150, "uusd"));
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::Purchase {
        number_of_tokens: Some(1),
    };
    let info = mock_info("C", &coins(150, "uusd"));
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let state = State {
        expiration: Expiration::AtHeight(mock_env().block.height + 1),
        price: coin(100, "uusd"),
        min_tokens_sold: Uint128::from(5u128),
        max_amount_per_wallet: 2,
        amount_sold: Uint128::from(4u128),
        amount_to_send: Uint128::from(360u128),
        amount_transferred: Uint128::zero(),
        recipient: Recipient::Addr("recipient".to_string()),
    };
    assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

    assert_eq!(
        vec![
            get_purchase(MOCK_TOKENS_FOR_SALE[0], "A"),
            get_purchase(MOCK_TOKENS_FOR_SALE[1], "A")
        ],
        PURCHASES.load(deps.as_ref().storage, "A").unwrap()
    );

    assert_eq!(
        vec![get_purchase(MOCK_TOKENS_FOR_SALE[2], "B"),],
        PURCHASES.load(deps.as_ref().storage, "B").unwrap()
    );

    assert_eq!(
        vec![get_purchase(MOCK_TOKENS_FOR_SALE[3], "C"),],
        PURCHASES.load(deps.as_ref().storage, "C").unwrap()
    );
    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[0]));
    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[1]));
    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[2]));
    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[3]));

    assert_eq!(
        NUMBER_OF_TOKENS_AVAILABLE
            .load(deps.as_ref().storage)
            .unwrap(),
        Uint128::new(3)
    );

    let mut env = mock_env();
    env.block.height += 1;

    // User B claims their own refund.
    let msg = ExecuteMsg::ClaimRefund {};
    let info = mock_info("B", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        Response::new()
            .add_attribute("action", "claim_refund")
            .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: "B".to_string(),
                amount: coins(150, "uusd"),
            })),
        res
    );

    assert!(!PURCHASES.has(deps.as_ref().storage, "B"));

    env.contract.address = Addr::unchecked(MOCK_CONDITIONS_NOT_MET_CONTRACT);
    deps.querier.tokens_left_to_burn = 7;
    let msg = ExecuteMsg::EndSale { limit: None };
    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    let refund_msgs: Vec<CosmosMsg> = vec![
        // All of A's payments grouped into one message.
        CosmosMsg::Bank(BankMsg::Send {
            to_address: "A".to_string(),
            amount: coins(300, "uusd"),
        }),
        CosmosMsg::Bank(BankMsg::Send {
            to_address: "C".to_string(),
            amount: coins(150, "uusd"),
        }),
    ];
    let burn_msgs: Vec<CosmosMsg> = vec![
        get_burn_message(MOCK_TOKENS_FOR_SALE[0]),
        get_burn_message(MOCK_TOKENS_FOR_SALE[1]),
        get_burn_message(MOCK_TOKENS_FOR_SALE[2]),
        get_burn_message(MOCK_TOKENS_FOR_SALE[3]),
        // Tokens that were not sold.
        get_burn_message(MOCK_TOKENS_FOR_SALE[4]),
        get_burn_message(MOCK_TOKENS_FOR_SALE[5]),
        get_burn_message(MOCK_TOKENS_FOR_SALE[6]),
    ];

    assert_eq!(
        Response::new()
            .add_attribute("action", "issue_refunds_and_burn_tokens")
            .add_messages(refund_msgs)
            .add_messages(burn_msgs),
        res
    );

    assert!(!PURCHASES.has(deps.as_ref().storage, "A"));
    assert!(!PURCHASES.has(deps.as_ref().storage, "C"));

    // Burned tokens have been removed.
    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[4]));
    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[5]));
    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[6]));

    deps.querier.tokens_left_to_burn = 0;
    let _res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert!(STATE.may_load(deps.as_mut().storage).unwrap().is_none());
    assert_eq!(
        NUMBER_OF_TOKENS_AVAILABLE
            .load(deps.as_ref().storage)
            .unwrap(),
        Uint128::zero()
    );
}

#[test]
fn test_integration_conditions_met() {
    let mut deps = mock_dependencies_custom(&[]);
    deps.querier.contract_address = MOCK_CONDITIONS_MET_CONTRACT.to_string();
    let modules = vec![Module {
        module_type: RATES.to_owned(),
        address: AndrAddress {
            identifier: MOCK_RATES_CONTRACT.to_owned(),
        },
        is_mutable: false,
    }];
    init(deps.as_mut(), Some(modules));
    let mut env = mock_env();
    env.contract.address = Addr::unchecked(MOCK_CONDITIONS_MET_CONTRACT);

    // Mint all tokens.
    for &token_id in MOCK_TOKENS_FOR_SALE {
        let _res = mint(deps.as_mut(), token_id).unwrap();
        assert!(AVAILABLE_TOKENS.has(deps.as_ref().storage, token_id));
    }

    let msg = ExecuteMsg::StartSale {
        expiration: Expiration::AtHeight(mock_env().block.height + 1),
        price: coin(100, "uusd"),
        min_tokens_sold: Uint128::from(3u128),
        max_amount_per_wallet: Some(2),
        recipient: Recipient::Addr("recipient".to_string()),
    };

    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let msg = ExecuteMsg::Purchase {
        number_of_tokens: Some(1),
    };
    let info = mock_info("A", &coins(150, "uusd"));
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::Purchase {
        number_of_tokens: Some(1),
    };
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let msg = ExecuteMsg::Purchase {
        number_of_tokens: Some(1),
    };
    let info = mock_info("B", &coins(150, "uusd"));
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let msg = ExecuteMsg::Purchase {
        number_of_tokens: Some(1),
    };
    let info = mock_info("C", &coins(150, "uusd"));
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let msg = ExecuteMsg::Purchase {
        number_of_tokens: Some(1),
    };
    let info = mock_info("D", &coins(150, "uusd"));
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let mut state = State {
        expiration: Expiration::AtHeight(mock_env().block.height + 1),
        price: coin(100, "uusd"),
        min_tokens_sold: Uint128::from(3u128),
        max_amount_per_wallet: 2,
        amount_sold: Uint128::from(5u128),
        amount_to_send: Uint128::from(450u128),
        amount_transferred: Uint128::zero(),
        recipient: Recipient::Addr("recipient".to_string()),
    };
    assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

    assert_eq!(
        vec![
            get_purchase(MOCK_TOKENS_FOR_SALE[0], "A"),
            get_purchase(MOCK_TOKENS_FOR_SALE[1], "A")
        ],
        PURCHASES.load(deps.as_ref().storage, "A").unwrap()
    );

    assert_eq!(
        vec![get_purchase(MOCK_TOKENS_FOR_SALE[2], "B"),],
        PURCHASES.load(deps.as_ref().storage, "B").unwrap()
    );
    assert_eq!(
        vec![get_purchase(MOCK_TOKENS_FOR_SALE[3], "C"),],
        PURCHASES.load(deps.as_ref().storage, "C").unwrap()
    );
    assert_eq!(
        vec![get_purchase(MOCK_TOKENS_FOR_SALE[4], "D"),],
        PURCHASES.load(deps.as_ref().storage, "D").unwrap()
    );
    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[0]));
    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[1]));
    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[2]));
    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[3]));
    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[4]));

    env.block.height += 1;
    env.contract.address = Addr::unchecked(MOCK_CONDITIONS_MET_CONTRACT);

    let msg = ExecuteMsg::EndSale { limit: Some(1) };
    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "transfer_tokens_and_send_funds")
            .add_message(get_transfer_message(MOCK_TOKENS_FOR_SALE[0], "A"))
            .add_submessages(get_rates_messages()),
        res
    );

    assert_eq!(
        vec![get_purchase(MOCK_TOKENS_FOR_SALE[1], "A")],
        PURCHASES.load(deps.as_ref().storage, "A").unwrap()
    );

    state.amount_transferred += Uint128::from(1u128);
    assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

    let msg = ExecuteMsg::EndSale { limit: Some(2) };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "transfer_tokens_and_send_funds")
            .add_message(get_transfer_message(MOCK_TOKENS_FOR_SALE[1], "A"))
            .add_message(get_transfer_message(MOCK_TOKENS_FOR_SALE[2], "B"))
            .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: MOCK_ROYALTY_RECIPIENT.to_owned(),
                amount: vec![Coin {
                    // Royalty of 10% for A and B combined
                    amount: Uint128::from(20u128),
                    denom: "uusd".to_string(),
                }],
            }))
            .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: MOCK_TAX_RECIPIENT.to_owned(),
                amount: vec![Coin {
                    // Combined tax for both A and B
                    amount: Uint128::from(100u128),
                    denom: "uusd".to_string(),
                }],
            })),
        res
    );

    assert!(!PURCHASES.has(deps.as_ref().storage, "A"),);
    assert!(!PURCHASES.has(deps.as_ref().storage, "B"),);
    assert!(PURCHASES.has(deps.as_ref().storage, "C"),);
    assert!(PURCHASES.has(deps.as_ref().storage, "D"),);

    state.amount_transferred += Uint128::from(2u128);
    assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

    let msg = ExecuteMsg::EndSale { limit: None };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    assert!(!PURCHASES.has(deps.as_ref().storage, "C"),);
    assert!(!PURCHASES.has(deps.as_ref().storage, "D"),);

    assert_eq!(
        Response::new()
            .add_attribute("action", "transfer_tokens_and_send_funds")
            .add_message(get_transfer_message(MOCK_TOKENS_FOR_SALE[3], "C"))
            .add_message(get_transfer_message(MOCK_TOKENS_FOR_SALE[4], "D"))
            .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: MOCK_ROYALTY_RECIPIENT.to_owned(),
                amount: vec![Coin {
                    // Royalty of 10% for C and D combined
                    amount: Uint128::from(20u128),
                    denom: "uusd".to_string(),
                }],
            }))
            .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: MOCK_TAX_RECIPIENT.to_owned(),
                amount: vec![Coin {
                    // Combined tax for both C and D
                    amount: Uint128::from(100u128),
                    denom: "uusd".to_string(),
                }],
            })),
        res
    );

    state.amount_transferred += Uint128::from(2u128);
    assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

    let msg = ExecuteMsg::EndSale { limit: None };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    assert_eq!(3, res.messages.len());

    assert_eq!(
        Response::new()
            .add_attribute("action", "transfer_tokens_and_send_funds")
            // Now that all tokens have been transfered, can send the funds to recipient.
            .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: "recipient".to_string(),
                amount: coins(450u128, "uusd")
            }))
            // Burn tokens that were not purchased
            .add_message(get_burn_message(MOCK_TOKENS_FOR_SALE[5]))
            .add_message(get_burn_message(MOCK_TOKENS_FOR_SALE[6])),
        res
    );

    state.amount_to_send = Uint128::zero();
    assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

    // Burned tokens removed.
    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[5]));
    assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[6]));

    deps.querier.tokens_left_to_burn = 0;
    let _res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert!(STATE.may_load(deps.as_mut().storage).unwrap().is_none());
    assert_eq!(
        NUMBER_OF_TOKENS_AVAILABLE
            .load(deps.as_ref().storage)
            .unwrap(),
        Uint128::zero()
    );
}

#[test]
fn test_end_sale_single_purchase() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    STATE
        .save(
            deps.as_mut().storage,
            &State {
                expiration: Expiration::AtHeight(mock_env().block.height - 1),
                price: coin(100, "uusd"),
                min_tokens_sold: Uint128::from(1u128),
                max_amount_per_wallet: 5,
                amount_sold: Uint128::from(1u128),
                amount_to_send: Uint128::from(100u128),
                amount_transferred: Uint128::zero(),
                recipient: Recipient::Addr("recipient".to_string()),
            },
        )
        .unwrap();

    PURCHASES
        .save(
            deps.as_mut().storage,
            "A",
            &vec![Purchase {
                token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
                purchaser: "A".to_string(),
                tax_amount: Uint128::zero(),
                msgs: vec![],
            }],
        )
        .unwrap();

    let msg = ExecuteMsg::EndSale { limit: None };
    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "transfer_tokens_and_send_funds")
            // Burn tokens that were not purchased
            .add_message(get_transfer_message(MOCK_TOKENS_FOR_SALE[0], "A")),
        res
    );
}

#[test]
fn test_end_sale_all_tokens_sold() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    STATE
        .save(
            deps.as_mut().storage,
            &State {
                // Sale has not expired yet.
                expiration: Expiration::AtHeight(mock_env().block.height + 1),
                price: coin(100, "uusd"),
                min_tokens_sold: Uint128::from(1u128),
                max_amount_per_wallet: 5,
                amount_sold: Uint128::from(1u128),
                amount_to_send: Uint128::from(100u128),
                amount_transferred: Uint128::zero(),
                recipient: Recipient::Addr("recipient".to_string()),
            },
        )
        .unwrap();

    PURCHASES
        .save(
            deps.as_mut().storage,
            "A",
            &vec![Purchase {
                token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
                purchaser: "A".to_string(),
                tax_amount: Uint128::zero(),
                msgs: vec![],
            }],
        )
        .unwrap();

    NUMBER_OF_TOKENS_AVAILABLE
        .save(deps.as_mut().storage, &Uint128::zero())
        .unwrap();

    let msg = ExecuteMsg::EndSale { limit: None };
    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "transfer_tokens_and_send_funds")
            // Burn tokens that were not purchased
            .add_message(get_transfer_message(MOCK_TOKENS_FOR_SALE[0], "A")),
        res
    );
}

#[test]
fn test_end_sale_limit_zero() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut(), None);

    STATE
        .save(
            deps.as_mut().storage,
            &State {
                expiration: Expiration::AtHeight(mock_env().block.height - 1),
                price: coin(100, "uusd"),
                min_tokens_sold: Uint128::from(1u128),
                max_amount_per_wallet: 5,
                amount_sold: Uint128::from(1u128),
                amount_to_send: Uint128::from(100u128),
                amount_transferred: Uint128::zero(),
                recipient: Recipient::Addr("recipient".to_string()),
            },
        )
        .unwrap();
    NUMBER_OF_TOKENS_AVAILABLE
        .save(deps.as_mut().storage, &Uint128::new(1))
        .unwrap();

    PURCHASES
        .save(
            deps.as_mut().storage,
            "A",
            &vec![Purchase {
                token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
                purchaser: "A".to_string(),
                tax_amount: Uint128::zero(),
                msgs: vec![],
            }],
        )
        .unwrap();

    let msg = ExecuteMsg::EndSale { limit: Some(0) };
    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::LimitMustNotBeZero {}, res.unwrap_err());
}

#[test]
fn test_validate_andr_addresses_nonexisting_module() {
    let mut deps = mock_dependencies_custom(&[]);
    let msg = InstantiateMsg {
        token_address: AndrAddress {
            identifier: "e".to_owned(),
        },
        modules: Some(vec![Module {
            module_type: "address_list".to_string(),
            is_mutable: true,
            address: AndrAddress {
                identifier: "z".to_string(),
            },
        }]),
        can_mint_after_sale: true,
    };

    let info = mock_info("owner", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::UpdateMissionContract {
        address: MOCK_MISSION_CONTRACT.to_owned(),
    });

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidComponent {
            name: "z".to_string()
        },
        res.unwrap_err()
    );
}

#[test]
fn test_update_mission_contract_nonexisting_address() {
    let mut deps = mock_dependencies_custom(&[]);
    let msg = InstantiateMsg {
        token_address: AndrAddress {
            identifier: "z".to_owned(),
        },
        modules: None,
        can_mint_after_sale: true,
    };

    let info = mock_info("owner", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::UpdateMissionContract {
        address: MOCK_MISSION_CONTRACT.to_owned(),
    });

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidComponent {
            name: "z".to_string()
        },
        res.unwrap_err()
    );
}

#[test]
fn test_validate_andr_addresses_regular_address() {
    let mut deps = mock_dependencies_custom(&[]);
    let msg = InstantiateMsg {
        token_address: AndrAddress {
            identifier: "terra1asdf1ssdfadf".to_owned(),
        },
        modules: None,
        can_mint_after_sale: true,
    };

    let info = mock_info("owner", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::UpdateMissionContract {
        address: MOCK_MISSION_CONTRACT.to_owned(),
    });

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "update_mission_contract")
            .add_attribute("address", MOCK_MISSION_CONTRACT),
        res
    );
}

#[test]
fn test_addresslist() {
    let mut deps = mock_dependencies_custom(&[]);
    let modules = vec![Module {
        module_type: ADDRESS_LIST.to_owned(),
        address: AndrAddress {
            identifier: MOCK_ADDRESSLIST_CONTRACT.to_owned(),
        },
        is_mutable: false,
    }];
    let msg = InstantiateMsg {
        token_address: AndrAddress {
            identifier: MOCK_TOKEN_CONTRACT.to_owned(),
        },
        modules: Some(modules),
        can_mint_after_sale: true,
    };

    let info = mock_info("mission_contract", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // Update mission contract
    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::UpdateMissionContract {
        address: "mission_contract".to_string(),
    });

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        Response::new()
            .add_attribute("action", "update_mission_contract")
            .add_attribute("address", "mission_contract"),
        res
    );

    // Not whitelisted user
    let msg = ExecuteMsg::Purchase {
        number_of_tokens: None,
    };
    let info = mock_info("not_whitelisted", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::Std(StdError::generic_err(
            "Querier contract error: InvalidAddress"
        )),
        res.unwrap_err()
    );
}
