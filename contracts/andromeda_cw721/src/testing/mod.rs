use cosmwasm_std::{
    attr, coin, coins, to_binary, Coin, CosmosMsg, DepsMut, Env, Event, Response, StdError, SubMsg,
    WasmMsg,
};

use crate::contract::*;
use andromeda_protocol::{
    communication::{
        hooks::{AndromedaHook, OnFundsTransferResponse},
        modules::{InstantiateType, Module, ModuleType},
    },
    cw721::{ExecuteMsg, InstantiateMsg, QueryMsg, TokenExtension, TransferAgreement},
    cw721_offers::ExecuteMsg as OffersExecuteMsg,
    error::ContractError,
    rates::Funds,
    receipt::{ExecuteMsg as ReceiptExecuteMsg, Receipt},
    testing::mock_querier::{
        bank_sub_msg, mock_dependencies_custom, MOCK_ADDRESSLIST_CONTRACT, MOCK_OFFERS_CONTRACT,
        MOCK_RATES_CONTRACT, MOCK_RATES_RECIPIENT, MOCK_RECEIPT_CONTRACT,
    },
};
use cosmwasm_std::{
    from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Uint128,
};
use cw721::{NftInfoResponse, OwnerOfResponse};
use cw721_base::MintMsg;

const MINTER: &str = "minter";
const SYMBOL: &str = "TT";
const NAME: &str = "TestToken";

fn init_setup(deps: DepsMut, env: Env, modules: Option<Vec<Module>>) {
    let info = mock_info(MINTER, &[]);
    let inst_msg = InstantiateMsg {
        name: NAME.to_string(),
        symbol: SYMBOL.to_string(),
        minter: MINTER.to_string(),
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
fn test_transfer_nft() {
    let token_id = String::from("testtoken");
    let creator = String::from("creator");
    let mut deps = mock_dependencies(&[]);
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
            transfer_agreement: None,
            metadata: None,
            archived: false,
            pricing: None,
        },
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
    let mut deps = mock_dependencies(&[]);
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
            transfer_agreement: Some(TransferAgreement {
                amount: agreed_amount.clone(),
                purchaser: purchaser.to_string(),
            }),
            metadata: None,
            archived: false,
            pricing: None,
        },
    );

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
fn test_archive() {
    let token_id = String::from("testtoken");
    let creator = String::from("creator");
    let mut deps = mock_dependencies(&[]);
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
            transfer_agreement: None,
            metadata: None,
            archived: false,
            pricing: None,
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

    let query_msg = QueryMsg::NftInfo { token_id };
    let query_resp = query(deps.as_ref(), env, query_msg).unwrap();
    let resp: NftInfoResponse<TokenExtension> = from_binary(&query_resp).unwrap();
    assert!(resp.extension.archived)
}

#[test]
fn test_burn() {
    let token_id = String::from("testtoken");
    let creator = String::from("creator");
    let mut deps = mock_dependencies(&[]);
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
            transfer_agreement: None,
            metadata: None,
            archived: false,
            pricing: None,
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
    let mut deps = mock_dependencies(&[]);
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
            transfer_agreement: None,
            metadata: None,
            archived: true,
            pricing: None,
        },
    );

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
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let agreement = TransferAgreement {
        purchaser: String::from("purchaser"),
        amount: Coin {
            amount: Uint128::from(100u64),
            denom: "uluna".to_string(),
        },
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
            transfer_agreement: None,
            metadata: None,
            archived: false,
            pricing: None,
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

    let query_msg = QueryMsg::NftInfo { token_id };
    let query_resp = query(deps.as_ref(), env, query_msg).unwrap();
    let resp: NftInfoResponse<TokenExtension> = from_binary(&query_resp).unwrap();
    assert_eq!(resp.extension.transfer_agreement, Some(agreement))
}

#[test]
fn test_update_pricing() {
    let token_id = String::from("testtoken");
    let creator = String::from("creator");
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let price = Coin {
        amount: Uint128::from(100u64),
        denom: String::from("uluna"),
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
            transfer_agreement: None,
            metadata: None,
            archived: false,
            pricing: None,
        },
    );

    let msg = ExecuteMsg::UpdatePricing {
        token_id: token_id.clone(),
        price: Some(price.clone()),
    };

    let unauth_info = mock_info("anyone", &[]);
    assert_eq!(
        execute(deps.as_mut(), env.clone(), unauth_info, msg.clone()).unwrap_err(),
        ContractError::Unauthorized {}
    );

    let info = mock_info(creator.as_str(), &[]);
    assert!(execute(deps.as_mut(), env.clone(), info, msg).is_ok());

    let query_msg = QueryMsg::NftInfo { token_id };
    let query_resp = query(deps.as_ref(), env, query_msg).unwrap();
    let resp: NftInfoResponse<TokenExtension> = from_binary(&query_resp).unwrap();
    assert_eq!(resp.extension.pricing, Some(price))
}

#[test]
fn test_modules() {
    // TODO: Test InstantiateType::New() when Fetch contract works.
    let modules: Vec<Module> = vec![
        Module {
            module_type: ModuleType::Receipt,
            instantiate: InstantiateType::Address(MOCK_RECEIPT_CONTRACT.into()),
        },
        Module {
            module_type: ModuleType::Rates,
            instantiate: InstantiateType::Address(MOCK_RATES_CONTRACT.into()),
        },
        Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address(MOCK_ADDRESSLIST_CONTRACT.into()),
        },
    ];

    let mut deps = mock_dependencies_custom(&coins(100, "uusd"));

    let token_id = String::from("testtoken");
    let creator = String::from("creator");
    let env = mock_env();
    let agreement = TransferAgreement {
        purchaser: String::from("purchaser"),
        amount: Coin {
            amount: Uint128::from(100u64),
            denom: "uusd".to_string(),
        },
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
            transfer_agreement: None,
            metadata: None,
            archived: false,
            pricing: None,
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
fn test_accept_offer() {
    let modules: Vec<Module> = vec![Module {
        module_type: ModuleType::Offers,
        instantiate: InstantiateType::Address(MOCK_OFFERS_CONTRACT.into()),
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
            transfer_agreement: None,
            metadata: None,
            archived: false,
            pricing: None,
        },
    );

    let msg = ExecuteMsg::AcceptOffer {
        token_id: token_id.clone(),
    };

    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

    let info = mock_info(&creator, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: MOCK_OFFERS_CONTRACT.to_string(),
        funds: vec![],
        msg: to_binary(&OffersExecuteMsg::AcceptOffer {
            token_id: token_id.clone(),
            token_owner: creator,
        })
        .unwrap(),
    });
    assert_eq!(Response::new().add_message(msg), res);

    let query_msg = QueryMsg::OwnerOf {
        token_id,
        include_expired: None,
    };
    let query_resp = query(deps.as_ref(), mock_env(), query_msg).unwrap();
    let resp: OwnerOfResponse = from_binary(&query_resp).unwrap();
    assert_eq!(resp.owner, "purchaser");
}

#[test]
fn test_accept_offer_existing_transfer_agreement() {
    let modules: Vec<Module> = vec![Module {
        module_type: ModuleType::Offers,
        instantiate: InstantiateType::Address(MOCK_OFFERS_CONTRACT.into()),
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
            transfer_agreement: Some(TransferAgreement {
                amount: coin(0, "uusd"),
                purchaser: "anyone".to_string(),
            }),
            metadata: None,
            archived: false,
            pricing: None,
        },
    );

    let msg = ExecuteMsg::AcceptOffer { token_id };
    let info = mock_info(&creator, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    assert_eq!(ContractError::TransferAgreementExists {}, res.unwrap_err());
}
