use cosmwasm_std::{Coin, DepsMut, Env};

use crate::contract::*;
use andromeda_protocol::communication::modules::Module;
use andromeda_protocol::{
    cw721::{ExecuteMsg, InstantiateMsg, QueryMsg, TokenExtension, TransferAgreement},
    error::ContractError,
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
    assert!(execute(deps.as_mut(), env.clone(), info, transfer_msg.clone()).is_ok());

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
            publisher: creator.clone(),
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
    assert!(execute(deps.as_mut(), env.clone(), info, transfer_msg.clone()).is_ok());

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
    assert!(execute(deps.as_mut(), env.clone(), info, msg.clone()).is_ok());

    let query_msg = QueryMsg::NftInfo { token_id };
    let query_resp = query(deps.as_ref(), env, query_msg).unwrap();
    let resp: NftInfoResponse<TokenExtension> = from_binary(&query_resp).unwrap();
    assert!(resp.extension.archived)
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

    let msg = ExecuteMsg::Burn {
        token_id: token_id.clone(),
    };

    let info = mock_info(creator.as_str(), &[]);
    assert_eq!(
        execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err(),
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
    assert!(execute(deps.as_mut(), env.clone(), info, msg.clone()).is_ok());

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
    assert!(execute(deps.as_mut(), env.clone(), info, msg.clone()).is_ok());

    let query_msg = QueryMsg::NftInfo { token_id };
    let query_resp = query(deps.as_ref(), env, query_msg).unwrap();
    let resp: NftInfoResponse<TokenExtension> = from_binary(&query_resp).unwrap();
    assert_eq!(resp.extension.pricing, Some(price))
}
