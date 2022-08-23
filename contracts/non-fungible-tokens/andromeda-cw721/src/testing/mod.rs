use cosmwasm_std::{
    attr, coin, coins, from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, Event, Response, StdError, SubMsg,
    Uint128, WasmMsg,
};

use common::{
    ado_base::{
        hooks::{AndromedaHook, OnFundsTransferResponse},
        modules::{Module, ADDRESS_LIST, OFFERS, RATES, RECEIPT},
        AndromedaMsg, AndromedaQuery,
    },
    app::AndrAddress,
    error::ContractError,
    primitive::{PrimitivePointer, Value},
    Funds,
};

use crate::{contract::*, state::ANDR_MINTER};
use andromeda_modules::receipt::{ExecuteMsg as ReceiptExecuteMsg, Receipt};
use andromeda_non_fungible_tokens::{
    cw721::{ExecuteMsg, InstantiateMsg, QueryMsg, TokenExtension, TransferAgreement},
    cw721_offers::ExecuteMsg as OffersExecuteMsg,
};
use andromeda_testing::testing::mock_querier::{
    bank_sub_msg, mock_dependencies_custom, MOCK_ADDRESSLIST_CONTRACT, MOCK_OFFERS_CONTRACT,
    MOCK_PRIMITIVE_CONTRACT, MOCK_RATES_CONTRACT, MOCK_RATES_RECIPIENT, MOCK_RECEIPT_CONTRACT,
};
use cw721::{AllNftInfoResponse, OwnerOfResponse};
use cw721_base::MintMsg;

const MINTER: &str = "minter";
const SYMBOL: &str = "TT";
const NAME: &str = "TestToken";

fn init_setup(deps: DepsMut, env: Env, modules: Option<Vec<Module>>) {
    let info = mock_info(MINTER, &[]);
    let inst_msg = InstantiateMsg {
        name: NAME.to_string(),
        symbol: SYMBOL.to_string(),
        minter: AndrAddress {
            identifier: MINTER.to_string(),
        },
        modules,
    };

    instantiate(deps, env, info, inst_msg).unwrap();
}

fn mint_token(deps: DepsMut, env: Env, token_id: String, owner: String, extension: TokenExtension) {
    let info = mock_info(MINTER, &[]);
    let mint_msg = MintMsg {
        token_id,
        owner,
        token_uri: None,
        extension,
    };
    execute(deps, env, info, ExecuteMsg::Mint(Box::new(mint_msg))).unwrap();
}

#[test]
fn test_andr_query() {
    let mut deps = mock_dependencies();
    init_setup(deps.as_mut(), mock_env(), None);

    let msg = QueryMsg::AndrQuery(AndromedaQuery::Owner {});
    let res = query(deps.as_ref(), mock_env(), msg);
    // Test that the query is hooked up correctly.
    assert!(res.is_ok())
}

/*
 * TODO: Remove when we are happy with IstantiateType replacement.
 * #[test]
fn test_instantiate_modules() {
    let receipt_msg = to_binary(&ReceiptInstantiateMsg {
        minter: "minter".to_string(),
        operators: None,
    })
    .unwrap();
    let rates_msg = to_binary(&RatesInstantiateMsg { rates: vec![] }).unwrap();
    let addresslist_msg = to_binary(&AddressListInstantiateMsg {
        operators: vec![],
        is_inclusive: true,
    })
    .unwrap();
    let modules: Vec<Module> = vec![
        Module {
            module_type: RECEIPT.to_owned(),
            instantiate: InstantiateType::New(receipt_msg.clone()),
            is_mutable: false,
        },
        Module {
            module_type: RATES.to_owned(),
            instantiate: InstantiateType::New(rates_msg.clone()),
            is_mutable: false,
        },
        Module {
            module_type: ADDRESS_LIST.to_owned(),
            instantiate: InstantiateType::New(addresslist_msg.clone()),
            is_mutable: false,
        },
    ];
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("sender", &[]);

    let instantiate_msg = InstantiateMsg {
        name: "Name".into(),
        symbol: "Symbol".into(),
        minter: AndrAddress {
            identifier: "minter".to_string(),
        },
        modules: Some(modules),
        primitive_contract: MOCK_PRIMITIVE_CONTRACT.to_owned(),
    };

    let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

    assert_eq!(
        "sender",
        ADOContract::default()
            .owner
            .load(deps.as_mut().storage)
            .unwrap()
    );
    assert_eq!(
        "cw721",
        ADOContract::default()
            .ado_type
            .load(deps.as_mut().storage)
            .unwrap()
    );

    let msgs: Vec<SubMsg> = vec![
        SubMsg {
            id: 1,
            reply_on: ReplyOn::Always,
            msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: None,
                code_id: 1,
                msg: receipt_msg,
                funds: vec![],
                label: "Instantiate: receipt".to_string(),
            }),
            gas_limit: None,
        },
        SubMsg {
            id: 2,
            reply_on: ReplyOn::Always,
            msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: None,
                code_id: 2,
                msg: rates_msg,
                funds: vec![],
                label: "Instantiate: rates".to_string(),
            }),
            gas_limit: None,
        },
        SubMsg {
            id: 3,
            reply_on: ReplyOn::Always,
            msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: None,
                code_id: 3,
                msg: addresslist_msg,
                funds: vec![],
                label: "Instantiate: address_list".to_string(),
            }),
            gas_limit: None,
        },
    ];
    assert_eq!(
        Response::new()
            .add_attribute("action", "register_module")
            .add_attribute("action", "register_module")
            .add_attribute("action", "register_module")
            .add_attribute("method", "instantiate")
            .add_attribute("type", "cw721")
            .add_submessages(msgs),
        res
    );
}*/
#[test]
fn test_transfer_nft() {
    let token_id = String::from("testtoken");
    let creator = String::from("creator");
    let mut deps = mock_dependencies();
    let env = mock_env();
    init_setup(deps.as_mut(), env.clone(), None);
    assert_eq!(
        AndrAddress {
            identifier: MINTER.to_owned()
        },
        ANDR_MINTER.load(deps.as_ref().storage).unwrap()
    );
    mint_token(
        deps.as_mut(),
        env.clone(),
        token_id.clone(),
        creator.clone(),
        TokenExtension {
            description: None,
            name: String::default(),
            publisher: creator.clone(),
            image: String::from(""),
            image_data: None,
            external_url: None,
            animation_url: None,
            youtube_url: None,
            attributes: vec![],
        },
    );

    assert_eq!(
        MINTER,
        AndrCW721Contract::default()
            .minter
            .load(deps.as_ref().storage)
            .unwrap()
    );

    let transfer_msg = ExecuteMsg::TransferNft {
        recipient: Addr::unchecked("recipient").to_string(),
        token_id: token_id.clone(),
    };

    let unauth_info = mock_info("anyone", &[]);
    assert_eq!(
        execute(
            deps.as_mut(),
            env.clone(),
            unauth_info,
            transfer_msg.clone()
        )
        .unwrap_err(),
        ContractError::Unauthorized {}
    );

    let info = mock_info(creator.as_str(), &[]);
    assert!(execute(deps.as_mut(), env.clone(), info, transfer_msg).is_ok());

    let query_msg = QueryMsg::OwnerOf {
        token_id,
        include_expired: None,
    };
    let query_resp = query(deps.as_ref(), env, query_msg).unwrap();
    let resp: OwnerOfResponse = from_binary(&query_resp).unwrap();
    assert_eq!(resp.owner, String::from("recipient"))
}

#[test]
fn test_agreed_transfer_nft() {
    let token_id = String::from("testtoken");
    let creator = String::from("creator");
    let valid_info = mock_info(creator.as_str(), &[]);
    let mut deps = mock_dependencies();
    let env = mock_env();
    let agreed_amount = Coin {
        denom: "uluna".to_string(),
        amount: Uint128::from(100u64),
    };
    let purchaser = "purchaser";
    init_setup(deps.as_mut(), env.clone(), None);
    mint_token(
        deps.as_mut(),
        env.clone(),
        token_id.clone(),
        creator.clone(),
        TokenExtension {
            description: None,
            name: String::default(),
            publisher: creator,
            attributes: vec![],
            image: String::from(""),
            image_data: None,
            external_url: None,
            animation_url: None,
            youtube_url: None,
        },
    );

    let transfer_agreement_msg = ExecuteMsg::TransferAgreement {
        token_id: token_id.clone(),
        agreement: Some(TransferAgreement {
            amount: Value::Raw(agreed_amount.clone()),
            purchaser: purchaser.to_string(),
        }),
    };
    execute(
        deps.as_mut(),
        env.clone(),
        valid_info,
        transfer_agreement_msg,
    )
    .unwrap();

    let transfer_msg = ExecuteMsg::TransferNft {
        recipient: Addr::unchecked("recipient").to_string(),
        token_id: token_id.clone(),
    };

    let invalid_info = mock_info(purchaser, &[]);
    assert_eq!(
        execute(
            deps.as_mut(),
            env.clone(),
            invalid_info,
            transfer_msg.clone()
        )
        .unwrap_err(),
        ContractError::InsufficientFunds {}
    );

    let info = mock_info(purchaser, &[agreed_amount]);
    assert!(execute(deps.as_mut(), env.clone(), info, transfer_msg).is_ok());

    let query_msg = QueryMsg::OwnerOf {
        token_id,
        include_expired: None,
    };
    let query_resp = query(deps.as_ref(), env, query_msg).unwrap();
    let resp: OwnerOfResponse = from_binary(&query_resp).unwrap();
    assert_eq!(resp.owner, String::from("recipient"))
}

#[test]
fn test_agreed_transfer_token_doesnt_exist() {
    let token_id = String::from("testtoken");
    let creator = String::from("creator");
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let valid_info = mock_info(creator.as_str(), &[]);
    let purchaser = "purchaser";
    let agreement = TransferAgreement {
        amount: Value::Pointer(PrimitivePointer {
            address: AndrAddress {
                identifier: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            },
            key: Some("sell_amount".to_string()),
        }),
        purchaser: purchaser.to_string(),
    };
    init_setup(deps.as_mut(), env.clone(), None);

    let transfer_agreement = ExecuteMsg::TransferAgreement {
        token_id,
        agreement: Some(agreement),
    };
    let received = execute(deps.as_mut(), env, valid_info, transfer_agreement).unwrap_err();
    let expected = ContractError::Std(StdError::NotFound {
        kind: "cw721_base::state::TokenInfo<andromeda_non_fungible_tokens::cw721::TokenExtension>"
            .to_string(),
    });

    assert_eq!(received, expected)
}

#[test]
fn test_agreed_transfer_nft_primitive_pointer() {
    let token_id = String::from("testtoken");
    let creator = String::from("creator");
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let agreed_amount = Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(100u64),
    };
    let valid_info = mock_info(creator.as_str(), &[]);
    let purchaser = "purchaser";
    let agreement = TransferAgreement {
        amount: Value::Pointer(PrimitivePointer {
            address: AndrAddress {
                identifier: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            },
            key: Some("sell_amount".to_string()),
        }),
        purchaser: purchaser.to_string(),
    };
    init_setup(deps.as_mut(), env.clone(), None);
    mint_token(
        deps.as_mut(),
        env.clone(),
        token_id.clone(),
        creator.clone(),
        TokenExtension {
            description: None,
            name: String::default(),
            publisher: creator,
            attributes: vec![],
            image: String::from(""),
            image_data: None,
            external_url: None,
            animation_url: None,
            youtube_url: None,
        },
    );

    let transfer_agreement = ExecuteMsg::TransferAgreement {
        token_id: token_id.clone(),
        agreement: Some(agreement),
    };
    execute(deps.as_mut(), env.clone(), valid_info, transfer_agreement).unwrap();

    let transfer_msg = ExecuteMsg::TransferNft {
        recipient: Addr::unchecked("recipient").to_string(),
        token_id: token_id.clone(),
    };

    let invalid_info = mock_info(purchaser, &[]);
    assert_eq!(
        execute(
            deps.as_mut(),
            env.clone(),
            invalid_info,
            transfer_msg.clone()
        )
        .unwrap_err(),
        ContractError::InsufficientFunds {}
    );

    let info = mock_info(purchaser, &[agreed_amount.clone()]);
    let res = execute(deps.as_mut(), env.clone(), info, transfer_msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "creator".to_string(),
                amount: vec![agreed_amount]
            })
            .add_attribute("action", "transfer")
            .add_attribute("recipient", "recipient"),
        res
    );

    let query_msg = QueryMsg::OwnerOf {
        token_id,
        include_expired: None,
    };
    let query_resp = query(deps.as_ref(), env, query_msg).unwrap();
    let resp: OwnerOfResponse = from_binary(&query_resp).unwrap();
    assert_eq!(resp.owner, String::from("recipient"))
}

#[test]
fn test_agreed_transfer_nft_wildcard() {
    let token_id = String::from("testtoken");
    let creator = String::from("creator");
    let mut deps = mock_dependencies();
    let env = mock_env();
    let agreed_amount = Coin {
        denom: "uluna".to_string(),
        amount: Uint128::from(100u64),
    };
    let purchaser = "*";
    init_setup(deps.as_mut(), env.clone(), None);
    mint_token(
        deps.as_mut(),
        env.clone(),
        token_id.clone(),
        creator.clone(),
        TokenExtension {
            description: None,
            name: String::default(),
            publisher: creator.clone(),
            attributes: vec![],
            image: String::from(""),
            image_data: None,
            external_url: None,
            animation_url: None,
            youtube_url: None,
        },
    );

    // Update transfer agreement.
    let msg = ExecuteMsg::TransferAgreement {
        token_id: token_id.clone(),
        agreement: Some(TransferAgreement {
            amount: Value::Raw(agreed_amount.clone()),
            purchaser: purchaser.to_string(),
        }),
    };
    let _res = execute(deps.as_mut(), mock_env(), mock_info(&creator, &[]), msg).unwrap();

    // Transfer the nft
    let transfer_msg = ExecuteMsg::TransferNft {
        recipient: Addr::unchecked("recipient").to_string(),
        token_id: token_id.clone(),
    };

    let info = mock_info("anyone", &[agreed_amount]);
    let _res = execute(deps.as_mut(), env.clone(), info, transfer_msg).unwrap();

    let query_msg = QueryMsg::OwnerOf {
        token_id,
        include_expired: None,
    };
    let query_resp = query(deps.as_ref(), env, query_msg).unwrap();
    let resp: OwnerOfResponse = from_binary(&query_resp).unwrap();
    assert_eq!(resp.owner, String::from("recipient"))
}

#[test]
fn test_archive() {
    let token_id = String::from("testtoken");
    let creator = String::from("creator");
    let mut deps = mock_dependencies();
    let env = mock_env();
    init_setup(deps.as_mut(), env.clone(), None);
    mint_token(
        deps.as_mut(),
        env.clone(),
        token_id.clone(),
        creator.clone(),
        TokenExtension {
            description: None,
            name: String::default(),
            publisher: creator.clone(),
            attributes: vec![],
            image: String::from(""),
            image_data: None,
            external_url: None,
            animation_url: None,
            youtube_url: None,
        },
    );

    let msg = ExecuteMsg::Archive {
        token_id: token_id.clone(),
    };

    let unauth_info = mock_info("anyone", &[]);
    assert_eq!(
        execute(deps.as_mut(), env.clone(), unauth_info, msg.clone()).unwrap_err(),
        ContractError::Unauthorized {}
    );

    let info = mock_info(creator.as_str(), &[]);
    assert!(execute(deps.as_mut(), env.clone(), info, msg).is_ok());

    let query_msg = QueryMsg::IsArchived { token_id };
    let query_resp = query(deps.as_ref(), env, query_msg).unwrap();
    let resp: bool = from_binary(&query_resp).unwrap();
    assert!(resp)
}

#[test]
fn test_burn() {
    let token_id = String::from("testtoken");
    let creator = String::from("creator");
    let mut deps = mock_dependencies();
    let env = mock_env();
    init_setup(deps.as_mut(), env.clone(), None);
    mint_token(
        deps.as_mut(),
        env.clone(),
        token_id.clone(),
        creator.clone(),
        TokenExtension {
            description: None,
            name: String::default(),
            publisher: creator.clone(),
            attributes: vec![],
            image: String::from(""),
            image_data: None,
            external_url: None,
            animation_url: None,
            youtube_url: None,
        },
    );

    let msg = ExecuteMsg::Burn {
        token_id: token_id.clone(),
    };

    let unauth_info = mock_info("anyone", &[]);
    assert_eq!(
        execute(deps.as_mut(), env.clone(), unauth_info, msg.clone()).unwrap_err(),
        ContractError::Unauthorized {}
    );

    let info = mock_info(creator.as_str(), &[]);
    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

    assert_eq!(
        Response::default().add_attributes(vec![
            attr("action", "burn"),
            attr("token_id", &token_id),
            attr("sender", info.sender.to_string()),
        ]),
        res
    );

    let contract = AndrCW721Contract::default();
    assert_eq!(
        None,
        contract
            .tokens
            .may_load(deps.as_ref().storage, &token_id)
            .unwrap()
    );

    assert_eq!(0, contract.token_count.load(deps.as_ref().storage).unwrap());
}

#[test]
fn test_archived_check() {
    let token_id = String::from("testtoken");
    let creator = String::from("creator");
    let valid_info = mock_info(creator.as_str(), &[]);
    let mut deps = mock_dependencies();
    let env = mock_env();
    init_setup(deps.as_mut(), env.clone(), None);
    mint_token(
        deps.as_mut(),
        env.clone(),
        token_id.clone(),
        creator.clone(),
        TokenExtension {
            description: None,
            name: String::default(),
            publisher: creator.clone(),
            attributes: vec![],
            image: String::from(""),
            image_data: None,
            external_url: None,
            animation_url: None,
            youtube_url: None,
        },
    );

    let archive_msg = ExecuteMsg::Archive {
        token_id: token_id.clone(),
    };
    execute(deps.as_mut(), env.clone(), valid_info, archive_msg).unwrap();

    let msg = ExecuteMsg::Burn { token_id };

    let info = mock_info(creator.as_str(), &[]);
    assert_eq!(
        execute(deps.as_mut(), env, info, msg).unwrap_err(),
        ContractError::TokenIsArchived {}
    );
}

#[test]
fn test_transfer_agreement() {
    let token_id = String::from("testtoken");
    let creator = String::from("creator");
    let mut deps = mock_dependencies();
    let env = mock_env();
    let agreement = TransferAgreement {
        purchaser: String::from("purchaser"),
        amount: Value::Raw(Coin {
            amount: Uint128::from(100u64),
            denom: "uluna".to_string(),
        }),
    };
    init_setup(deps.as_mut(), env.clone(), None);
    mint_token(
        deps.as_mut(),
        env.clone(),
        token_id.clone(),
        creator.clone(),
        TokenExtension {
            description: None,
            name: String::default(),
            publisher: creator.clone(),
            attributes: vec![],
            image: String::from(""),
            image_data: None,
            external_url: None,
            animation_url: None,
            youtube_url: None,
        },
    );

    let msg = ExecuteMsg::TransferAgreement {
        token_id: token_id.clone(),
        agreement: Some(agreement.clone()),
    };

    let unauth_info = mock_info("anyone", &[]);
    assert_eq!(
        execute(deps.as_mut(), env.clone(), unauth_info, msg.clone()).unwrap_err(),
        ContractError::Unauthorized {}
    );

    let info = mock_info(creator.as_str(), &[]);
    assert!(execute(deps.as_mut(), env.clone(), info, msg).is_ok());

    let query_msg = QueryMsg::TransferAgreement { token_id };
    let query_resp = query(deps.as_ref(), env, query_msg).unwrap();
    let resp: Option<TransferAgreement> = from_binary(&query_resp).unwrap();
    assert!(resp.is_some());
    assert_eq!(resp, Some(agreement))
}

#[test]
fn test_modules() {
    let modules: Vec<Module> = vec![
        Module {
            module_type: RECEIPT.to_owned(),
            address: AndrAddress {
                identifier: MOCK_RECEIPT_CONTRACT.to_owned(),
            },
            is_mutable: false,
        },
        Module {
            module_type: RATES.to_owned(),
            address: AndrAddress {
                identifier: MOCK_RATES_CONTRACT.to_owned(),
            },
            is_mutable: false,
        },
        Module {
            module_type: ADDRESS_LIST.to_owned(),
            address: AndrAddress {
                identifier: MOCK_ADDRESSLIST_CONTRACT.to_owned(),
            },
            is_mutable: false,
        },
    ];

    let mut deps = mock_dependencies_custom(&coins(100, "uusd"));

    let token_id = String::from("testtoken");
    let creator = String::from("creator");
    let env = mock_env();
    let agreement = TransferAgreement {
        purchaser: String::from("purchaser"),
        amount: Value::Raw(Coin {
            amount: Uint128::from(100u64),
            denom: "uusd".to_string(),
        }),
    };
    init_setup(deps.as_mut(), env.clone(), Some(modules));
    mint_token(
        deps.as_mut(),
        env,
        token_id.clone(),
        creator.clone(),
        TokenExtension {
            description: None,
            name: String::default(),
            publisher: creator.clone(),
            attributes: vec![],
            image: String::from(""),
            image_data: None,
            external_url: None,
            animation_url: None,
            youtube_url: None,
        },
    );

    let msg = ExecuteMsg::TransferAgreement {
        token_id: token_id.clone(),
        agreement: Some(agreement),
    };

    let not_whitelisted_info = mock_info("not_whitelisted", &[]);
    let res = execute(deps.as_mut(), mock_env(), not_whitelisted_info, msg.clone());
    assert_eq!(
        ContractError::Std(StdError::generic_err(
            "Querier contract error: InvalidAddress"
        )),
        res.unwrap_err()
    );

    let info = mock_info("creator", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::TransferNft {
        token_id: token_id.clone(),
        recipient: "purchaser".into(),
    };

    // Tax not added by sender, remember that the contract holds 100 uusd which is enough to cover
    // the taxes in this case.
    let purchaser = mock_info("purchaser", &coins(100, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), purchaser, msg.clone());
    assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());

    // Add 10 for tax.
    let purchaser = mock_info("purchaser", &coins(100 + 10, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), purchaser, msg).unwrap();

    let receipt_msg: SubMsg = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: MOCK_RECEIPT_CONTRACT.to_string(),
        msg: to_binary(&ReceiptExecuteMsg::StoreReceipt {
            receipt: Receipt {
                events: vec![Event::new("Royalty"), Event::new("Tax")],
            },
        })
        .unwrap(),
        funds: vec![],
    }));

    let sub_msgs: Vec<SubMsg> = vec![
        // For royalty.
        bank_sub_msg(10, MOCK_RATES_RECIPIENT),
        // For tax.
        bank_sub_msg(10, MOCK_RATES_RECIPIENT),
        receipt_msg.clone(),
        bank_sub_msg(90, &creator),
    ];

    assert_eq!(
        Response::new()
            .add_attribute("action", "transfer")
            .add_attribute("recipient", "purchaser")
            .add_submessages(sub_msgs)
            .add_event(Event::new("Royalty"))
            .add_event(Event::new("Tax")),
        res
    );

    // Test the hook.
    let msg = QueryMsg::AndrHook(AndromedaHook::OnFundsTransfer {
        sender: "sender".to_string(),
        payload: to_binary(&token_id).unwrap(),
        amount: Funds::Native(coin(100, "uusd")),
    });

    let res: OnFundsTransferResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();

    let expected_response = OnFundsTransferResponse {
        msgs: vec![
            bank_sub_msg(10, MOCK_RATES_RECIPIENT),
            bank_sub_msg(10, MOCK_RATES_RECIPIENT),
            receipt_msg,
        ],
        leftover_funds: Funds::Native(coin(90, "uusd")),
        events: vec![Event::new("Royalty"), Event::new("Tax")],
    };
    assert_eq!(expected_response, res);
}

#[test]
fn test_transfer_with_offer() {
    let modules: Vec<Module> = vec![Module {
        module_type: OFFERS.to_owned(),
        address: AndrAddress {
            identifier: MOCK_OFFERS_CONTRACT.to_owned(),
        },
        is_mutable: false,
    }];

    let mut deps = mock_dependencies_custom(&coins(100, "uusd"));

    let token_id = String::from("testtoken");
    let creator = String::from("creator");
    let env = mock_env();
    init_setup(deps.as_mut(), env.clone(), Some(modules));
    mint_token(
        deps.as_mut(),
        env,
        token_id.clone(),
        creator.clone(),
        TokenExtension {
            description: None,
            name: String::default(),
            publisher: creator.clone(),
            attributes: vec![],
            image: String::from(""),
            image_data: None,
            external_url: None,
            animation_url: None,
            youtube_url: None,
        },
    );

    let msg = ExecuteMsg::TransferNft {
        recipient: "purchaser".to_string(),
        token_id: token_id.clone(),
    };
    let info = mock_info(&creator, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let msg: SubMsg = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: MOCK_OFFERS_CONTRACT.to_owned(),
        funds: vec![],
        msg: to_binary(&OffersExecuteMsg::AcceptOffer {
            token_id,
            recipient: creator,
        })
        .unwrap(),
    }));
    assert_eq!(
        Response::new()
            .add_submessage(msg)
            .add_attribute("action", "transfer")
            .add_attribute("recipient", "purchaser"),
        res
    );
}

#[test]
fn test_update_app_contract() {
    let mut deps = mock_dependencies_custom(&[]);

    let modules: Vec<Module> = vec![
        Module {
            module_type: ADDRESS_LIST.to_owned(),
            address: AndrAddress {
                identifier: MOCK_ADDRESSLIST_CONTRACT.to_owned(),
            },
            is_mutable: false,
        },
        Module {
            module_type: RATES.to_owned(),
            address: AndrAddress {
                identifier: "b".to_owned(),
            },
            is_mutable: false,
        },
    ];

    let info = mock_info("app_contract", &[]);
    let inst_msg = InstantiateMsg {
        name: NAME.to_string(),
        symbol: SYMBOL.to_string(),
        minter: AndrAddress {
            identifier: "e".to_string(),
        },
        modules: Some(modules),
    };

    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), inst_msg).unwrap();

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
fn test_update_app_contract_invalid_minter() {
    let mut deps = mock_dependencies_custom(&[]);

    let modules: Vec<Module> = vec![
        Module {
            module_type: ADDRESS_LIST.to_owned(),
            address: AndrAddress {
                identifier: MOCK_ADDRESSLIST_CONTRACT.to_owned(),
            },
            is_mutable: false,
        },
        Module {
            module_type: RATES.to_owned(),
            address: AndrAddress {
                identifier: "b".to_owned(),
            },
            is_mutable: false,
        },
    ];

    let info = mock_info("app_contract", &[]);
    let inst_msg = InstantiateMsg {
        name: NAME.to_string(),
        symbol: SYMBOL.to_string(),
        minter: AndrAddress {
            identifier: "k".to_string(),
        },
        modules: Some(modules),
    };

    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), inst_msg).unwrap();

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::UpdateAppContract {
        address: "app_contract".to_string(),
    });

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(
        ContractError::InvalidComponent {
            name: "k".to_string()
        },
        res.unwrap_err()
    );
}

#[test]
fn test_update_app_contract_invalid_module() {
    let mut deps = mock_dependencies_custom(&[]);

    let modules: Vec<Module> = vec![
        Module {
            module_type: ADDRESS_LIST.to_owned(),
            address: AndrAddress {
                identifier: MOCK_ADDRESSLIST_CONTRACT.to_owned(),
            },
            is_mutable: false,
        },
        Module {
            module_type: RATES.to_owned(),
            address: AndrAddress {
                identifier: "k".to_owned(),
            },
            is_mutable: false,
        },
    ];

    let info = mock_info("app_contract", &[]);
    let inst_msg = InstantiateMsg {
        name: NAME.to_string(),
        symbol: SYMBOL.to_string(),
        minter: AndrAddress {
            identifier: MINTER.to_string(),
        },
        modules: Some(modules),
    };

    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), inst_msg).unwrap();

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::UpdateAppContract {
        address: "app_contract".to_string(),
    });

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(
        ContractError::InvalidComponent {
            name: "k".to_string()
        },
        res.unwrap_err()
    );
}

#[test]
fn test_batch_mint() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info(MINTER, &[]);
    let inst_msg = InstantiateMsg {
        name: NAME.to_string(),
        symbol: SYMBOL.to_string(),
        minter: AndrAddress {
            identifier: MINTER.to_string(),
        },
        modules: None,
    };
    let owner = "owner";
    let mut mint_msgs: Vec<MintMsg<TokenExtension>> = Vec::new();

    let mut i: i32 = 0;
    while i < 5 {
        let mint_msg = MintMsg {
            token_id: i.to_string(),
            owner: owner.to_string(),
            token_uri: None,
            extension: TokenExtension {
                name: format!("Token {}", i),
                publisher: owner.to_string(),
                description: None,
                attributes: vec![],
                image: "Some URL".to_string(),
                image_data: None,
                external_url: None,
                youtube_url: None,
                animation_url: None,
            },
        };
        i += 1;
        mint_msgs.push(mint_msg)
    }

    instantiate(deps.as_mut(), mock_env(), info.clone(), inst_msg).unwrap();

    let msg: ExecuteMsg = ExecuteMsg::BatchMint { tokens: mint_msgs };

    let _resp = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let mut i: i32 = 0;
    while i < 5 {
        let query_msg = QueryMsg::AllNftInfo {
            token_id: i.to_string(),
            include_expired: None,
        };
        let query_resp = query(deps.as_ref(), mock_env(), query_msg).unwrap();
        let info: AllNftInfoResponse<TokenExtension> = from_binary(&query_resp).unwrap();
        assert_eq!(info.access.owner, owner.to_string());
        i += 1;
    }
}
