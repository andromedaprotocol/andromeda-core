use cosmwasm_std::{
    attr, coin, coins, from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, Coin, CosmosMsg, DepsMut, Env, Event, ReplyOn, Response, StdError, SubMsg,
    Uint128, WasmMsg,
};

use common::{
    ado_base::{
        hooks::{AndromedaHook, OnFundsTransferResponse},
        modules::{InstantiateType, Module, ModuleType},
    },
    error::ContractError,
    Funds,
};

use crate::contract::*;
use andromeda_protocol::{
    address_list::InstantiateMsg as AddressListInstantiateMsg,
    cw721::{ExecuteMsg, InstantiateMsg, QueryMsg, TokenExtension, TransferAgreement},
    cw721_offers::ExecuteMsg as OffersExecuteMsg,
    rates::InstantiateMsg as RatesInstantiateMsg,
    receipt::{ExecuteMsg as ReceiptExecuteMsg, InstantiateMsg as ReceiptInstantiateMsg, Receipt},
    testing::mock_querier::{
        bank_sub_msg, mock_dependencies_custom, MOCK_ADDRESSLIST_CONTRACT, MOCK_OFFERS_CONTRACT,
        MOCK_PRIMITIVE_CONTRACT, MOCK_RATES_CONTRACT, MOCK_RATES_RECIPIENT, MOCK_RECEIPT_CONTRACT,
    },
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
        primitive_contract: MOCK_PRIMITIVE_CONTRACT.to_owned(),
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
            module_type: ModuleType::Receipt,
            instantiate: InstantiateType::New(receipt_msg.clone()),
            is_mutable: false,
        },
        Module {
            module_type: ModuleType::Rates,
            instantiate: InstantiateType::New(rates_msg.clone()),
            is_mutable: false,
        },
        Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::New(addresslist_msg.clone()),
            is_mutable: false,
        },
    ];
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("sender", &[]);

    let instantiate_msg = InstantiateMsg {
        name: "Name".into(),
        symbol: "Symbol".into(),
        minter: "minter".into(),
        modules: Some(modules),
        primitive_contract: MOCK_PRIMITIVE_CONTRACT.to_owned(),
    };

    let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

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
    assert_eq!(Response::new().add_submessages(msgs), res);
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
    let modules: Vec<Module> = vec![
        Module {
            module_type: ModuleType::Receipt,
            instantiate: InstantiateType::Address(MOCK_RECEIPT_CONTRACT.into()),
            is_mutable: false,
        },
        Module {
            module_type: ModuleType::Rates,
            instantiate: InstantiateType::Address(MOCK_RATES_CONTRACT.into()),
            is_mutable: false,
        },
        Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address(MOCK_ADDRESSLIST_CONTRACT.into()),
            is_mutable: false,
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
fn test_transfer_with_offer() {
    let modules: Vec<Module> = vec![Module {
        module_type: ModuleType::Offers,
        instantiate: InstantiateType::Address(MOCK_OFFERS_CONTRACT.into()),
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
            transfer_agreement: None,
            metadata: None,
            archived: false,
            pricing: None,
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
