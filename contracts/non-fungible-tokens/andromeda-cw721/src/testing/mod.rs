use crate::{contract::*, state::TRANSFER_AGREEMENTS};
use andromeda_non_fungible_tokens::cw721::{
    BatchSendMsg, ExecuteMsg, InstantiateMsg, IsArchivedResponse, MintMsg, QueryMsg,
    TokenExtension, TransferAgreement,
};
use andromeda_std::testing::mock_querier;
use andromeda_std::{
    amp::addresses::AndrAddr,
    error::ContractError,
    testing::mock_querier::{mock_dependencies_custom, FAKE_VFS_PATH, MOCK_KERNEL_CONTRACT},
};
use cosmwasm_std::{
    attr, coin, from_json,
    testing::{message_info, mock_env},
    Addr, Binary, Coin, DepsMut, Env, Response, StdError, Uint128,
};
use cw721::{msg::AllNftInfoResponse, msg::OwnerOfResponse};
use cw721_base::traits::Cw721Query;
use rstest::rstest;

const MINTER: &str = "cosmwasm1h6t805h2vjfzpa3m9n8kyadyng9xf604nhvev8tf5qdg65jh3ruqwwm3zz";
const SYMBOL: &str = "TT";
const NAME: &str = "TestToken";
const _ADDRESS_LIST: &str = "addresslist";
// const RATES: &str = "rates";

fn init_setup(
    deps: &mut cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        crate::testing::mock_querier::WasmMockQuerier,
    >,
    env: Env,
) {
    let kernel_addr = deps.api.addr_make(MOCK_KERNEL_CONTRACT);
    let minter_addr = deps.api.addr_make("minter");
    let info = message_info(&minter_addr, &[]);
    let inst_msg = InstantiateMsg {
        name: NAME.to_string(),
        symbol: SYMBOL.to_string(),
        minter: AndrAddr::from_string(minter_addr.to_string()),
        kernel_address: kernel_addr.to_string(),
        owner: None,
    };

    instantiate(deps.as_mut(), env, info, inst_msg).unwrap();
}

fn mint_token(
    deps: DepsMut,
    env: Env,
    token_id: String,
    owner: impl Into<String>,
    _extension: TokenExtension,
) {
    let info = message_info(&Addr::unchecked(MINTER), &[]);
    let mint_msg = ExecuteMsg::Mint {
        token_id,
        owner: AndrAddr::from_string(owner.into()),
        token_uri: None,
    };
    execute(deps, env, info, mint_msg).unwrap();
}

#[test]
fn test_transfer_nft() {
    let token_id = String::from("testtoken");
    let creator = String::from("creator");
    let mut deps = mock_dependencies_custom(&[]);
    let creator_addr = deps.api.addr_make(&creator);

    let env = mock_env();
    init_setup(&mut deps, env.clone());
    mint_token(
        deps.as_mut(),
        env.clone(),
        token_id.clone(),
        creator_addr.to_string(),
        TokenExtension {},
    );

    let recipient_addr = deps.api.addr_make("recipient");
    let transfer_msg = ExecuteMsg::TransferNft {
        recipient: AndrAddr::from_string(recipient_addr.to_string()),
        token_id: token_id.clone(),
    };

    let anyone_addr = deps.api.addr_make("anyone");
    let unauth_info = message_info(&anyone_addr, &[]);
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

    let some_purchaser_addr = deps.api.addr_make("some_purchaser");
    TRANSFER_AGREEMENTS
        .save(
            deps.as_mut().storage,
            &token_id,
            &TransferAgreement {
                amount: coin(100u128, "uandr"),
                purchaser: some_purchaser_addr.to_string(),
            },
        )
        .unwrap();

    let info = message_info(&creator_addr, &[]);
    assert!(execute(deps.as_mut(), env.clone(), info, transfer_msg).is_ok());

    let query_msg = QueryMsg::OwnerOf {
        token_id: token_id.clone(),
        include_expired: None,
    };
    let query_resp = query(deps.as_ref(), env, query_msg).unwrap();
    let resp: OwnerOfResponse = from_json(query_resp).unwrap();
    assert_eq!(resp.owner, recipient_addr.to_string());

    let agreement = TRANSFER_AGREEMENTS
        .may_load(deps.as_ref().storage, &token_id)
        .unwrap();

    assert!(agreement.is_none());
}

#[test]
fn test_agreed_transfer_nft() {
    let token_id = String::from("testtoken");
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let creator_addr = deps.api.addr_make("creator");
    let valid_info = message_info(&creator_addr, &[]);

    let agreed_amount = Coin {
        denom: "uluna".to_string(),
        amount: Uint128::from(100u64),
    };
    let purchaser_addr = deps.api.addr_make("purchaser");
    init_setup(&mut deps, env.clone());
    mint_token(
        deps.as_mut(),
        env.clone(),
        token_id.clone(),
        creator_addr.to_string(),
        TokenExtension {},
    );

    let transfer_agreement_msg = ExecuteMsg::TransferAgreement {
        token_id: token_id.clone(),
        agreement: Some(TransferAgreement {
            amount: agreed_amount.clone(),
            purchaser: purchaser_addr.to_string(),
        }),
    };
    execute(
        deps.as_mut(),
        env.clone(),
        valid_info,
        transfer_agreement_msg,
    )
    .unwrap();

    let recipient_addr = deps.api.addr_make("recipient");
    let transfer_msg = ExecuteMsg::TransferNft {
        recipient: AndrAddr::from_string(recipient_addr.to_string()),
        token_id: token_id.clone(),
    };

    let invalid_info = message_info(&purchaser_addr, &[]);
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

    let info = message_info(&purchaser_addr, &[agreed_amount]);
    let _res = execute(deps.as_mut(), env.clone(), info, transfer_msg).unwrap();

    let query_msg = QueryMsg::OwnerOf {
        token_id,
        include_expired: None,
    };
    let query_resp = query(deps.as_ref(), env, query_msg).unwrap();
    let resp: OwnerOfResponse = from_json(query_resp).unwrap();
    assert_eq!(resp.owner, recipient_addr.to_string())
}

// TODO reenable wildcard functionality
// #[test]
// fn test_agreed_transfer_nft_wildcard() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let env = mock_env();
//     let token_id = String::from("testtoken");
//     let creator = String::from("creator");
//     let creator_addr = deps.api.addr_make(&creator);
//     let agreed_amount = Coin {
//         denom: "uluna".to_string(),
//         amount: Uint128::from(100u64),
//     };
//     let purchaser = "*";
//     init_setup(&mut deps, env.clone());
//     mint_token(
//         deps.as_mut(),
//         env.clone(),
//         token_id.clone(),
//         creator_addr.to_string(),
//         TokenExtension {},
//     );

//     // Update transfer agreement.
//     let msg = ExecuteMsg::TransferAgreement {
//         token_id: token_id.clone(),
//         agreement: Some(TransferAgreement {
//             amount: agreed_amount.clone(),
//             purchaser: purchaser.to_string(),
//         }),
//     };
//     let _res = execute(
//         deps.as_mut(),
//         mock_env(),
//         message_info(&creator_addr, &[]),
//         msg,
//     )
//     .unwrap();

//     // Transfer the nft
//     let recipient_addr = deps.api.addr_make("recipient");
//     let transfer_msg = ExecuteMsg::TransferNft {
//         recipient: AndrAddr::from_string(recipient_addr.to_string()),
//         token_id: token_id.clone(),
//     };

//     let anyone_addr = deps.api.addr_make("anyone");
//     let info = message_info(&anyone_addr, &[agreed_amount]);
//     let _res = execute(deps.as_mut(), env.clone(), info, transfer_msg).unwrap();

//     let query_msg = QueryMsg::OwnerOf {
//         token_id,
//         include_expired: None,
//     };
//     let query_resp = query(deps.as_ref(), env, query_msg).unwrap();
//     let resp: OwnerOfResponse = from_json(query_resp).unwrap();
//     assert_eq!(resp.owner, recipient_addr.to_string())
// }

#[test]
fn test_archive() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let token_id = String::from("testtoken");
    let creator = String::from("creator");
    let creator_addr = deps.api.addr_make(&creator);
    let anyone_addr = deps.api.addr_make("anyone");
    init_setup(&mut deps, env.clone());
    mint_token(
        deps.as_mut(),
        env.clone(),
        token_id.clone(),
        creator_addr.to_string(),
        TokenExtension {},
    );

    let msg = ExecuteMsg::Archive {
        token_id: token_id.clone(),
    };

    let unauth_info = message_info(&anyone_addr, &[]);
    assert_eq!(
        execute(deps.as_mut(), env.clone(), unauth_info, msg.clone()).unwrap_err(),
        ContractError::Unauthorized {}
    );

    let info = message_info(&creator_addr, &[]);
    assert!(execute(deps.as_mut(), env.clone(), info, msg).is_ok());

    let query_msg = QueryMsg::IsArchived { token_id };
    let query_resp = query(deps.as_ref(), env, query_msg).unwrap();
    let resp: IsArchivedResponse = from_json(query_resp).unwrap();
    assert!(resp.is_archived)
}

#[test]
fn test_burn() {
    let token_id = String::from("testtoken");
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let creator = deps.api.addr_make("creator");
    init_setup(&mut deps, env.clone());
    mint_token(
        deps.as_mut(),
        env.clone(),
        token_id.clone(),
        creator.to_string(),
        TokenExtension {},
    );

    let msg = ExecuteMsg::Burn {
        token_id: token_id.clone(),
    };

    let anyone_addr = deps.api.addr_make("anyone");
    let unauth_info = message_info(&anyone_addr, &[]);
    assert_eq!(
        execute(deps.as_mut(), env.clone(), unauth_info, msg.clone()).unwrap_err(),
        ContractError::Unauthorized {}
    );

    let info = message_info(&creator, &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    assert_eq!(
        Response::default().add_attributes(vec![
            attr("action", "burn"),
            attr("token_id", &token_id),
            attr("sender", info.sender.to_string()),
        ]),
        res
    );

    let contract = AndrCW721Contract;
    let tokens = contract
        .query_all_tokens(deps.as_ref(), &env, None, None)
        .unwrap();
    assert_eq!(tokens.tokens.len(), 0);
}

#[test]
fn test_archived_check() {
    let token_id = String::from("testtoken");
    let mut deps = mock_dependencies_custom(&[]);
    let creator = deps.api.addr_make("creator");
    let env = mock_env();
    let valid_info = message_info(&creator, &[]);

    init_setup(&mut deps, env.clone());
    mint_token(
        deps.as_mut(),
        env.clone(),
        token_id.clone(),
        creator.to_string(),
        TokenExtension {},
    );

    let archive_msg = ExecuteMsg::Archive {
        token_id: token_id.clone(),
    };
    execute(deps.as_mut(), env.clone(), valid_info, archive_msg).unwrap();

    let msg = ExecuteMsg::Burn { token_id };

    let info = message_info(&creator, &[]);
    assert_eq!(
        execute(deps.as_mut(), env.clone(), info, msg).unwrap_err(),
        ContractError::TokenIsArchived {}
    );
}

#[test]
fn test_transfer_agreement() {
    let token_id = String::from("testtoken");
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let creator = deps.api.addr_make("creator");
    let purchaser = deps.api.addr_make("purchaser");
    let agreement = TransferAgreement {
        purchaser: purchaser.to_string(),
        amount: Coin {
            amount: Uint128::from(100u64),
            denom: "uluna".to_string(),
        },
    };
    init_setup(&mut deps, env.clone());
    mint_token(
        deps.as_mut(),
        env.clone(),
        token_id.clone(),
        creator.to_string(),
        TokenExtension {},
    );

    let msg = ExecuteMsg::TransferAgreement {
        token_id: token_id.clone(),
        agreement: Some(agreement.clone()),
    };
    let anyone = deps.api.addr_make("anyone");
    let unauth_info = message_info(&anyone, &[]);
    assert_eq!(
        execute(deps.as_mut(), env.clone(), unauth_info, msg.clone()).unwrap_err(),
        ContractError::Unauthorized {}
    );

    let info = message_info(&Addr::unchecked(creator), &[]);
    assert!(execute(deps.as_mut(), env.clone(), info, msg).is_ok());

    let query_msg = QueryMsg::TransferAgreement { token_id };
    let query_resp = query(deps.as_ref(), env, query_msg).unwrap();
    let resp: Option<TransferAgreement> = from_json(query_resp).unwrap();
    assert!(resp.is_some());
    assert_eq!(resp, Some(agreement))
}

#[test]
fn test_update_app_contract_invalid_minter() {
    let mut deps = mock_dependencies_custom(&[]);
    let kernel_addr = deps.api.addr_make(MOCK_KERNEL_CONTRACT);
    let owner = deps.api.addr_make("owner");
    let app_contract = deps.api.addr_make("app_contract");
    let info = message_info(&app_contract, &[]);
    let fake_vfs = deps.api.addr_make(FAKE_VFS_PATH);
    let inst_msg = InstantiateMsg {
        name: NAME.to_string(),
        symbol: SYMBOL.to_string(),
        minter: AndrAddr::from_string(fake_vfs.to_string()),
        kernel_address: kernel_addr.to_string(),
        owner: Some(owner.to_string()),
    };

    instantiate(deps.as_mut(), mock_env(), info.clone(), inst_msg).unwrap();

    let msg = ExecuteMsg::Mint {
        token_id: "1".to_string(),
        owner: owner.into(),
        token_uri: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert!(res.is_err());
}

#[test]
fn test_batch_mint() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = message_info(&Addr::unchecked(MINTER), &[]);
    let kernel_addr = deps.api.addr_make(MOCK_KERNEL_CONTRACT);

    let inst_msg = InstantiateMsg {
        name: NAME.to_string(),
        symbol: SYMBOL.to_string(),
        minter: AndrAddr::from_string(MINTER),
        kernel_address: kernel_addr.to_string(),
        owner: None,
    };
    let owner = deps.api.addr_make("owner");
    let mut mint_msgs: Vec<MintMsg> = Vec::new();

    let mut i: i32 = 0;
    while i < 5 {
        let mint_msg = MintMsg {
            token_id: i.to_string(),
            owner: owner.clone().into(),
            token_uri: None,
            extension: TokenExtension {},
        };
        i += 1;
        mint_msgs.push(mint_msg)
    }

    instantiate(deps.as_mut(), mock_env(), info.clone(), inst_msg).unwrap();

    let msg: ExecuteMsg = ExecuteMsg::BatchMint { tokens: vec![] };

    let err = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::Std(StdError::generic_err("No tokens to mint"))
    );

    let msg: ExecuteMsg = ExecuteMsg::BatchMint { tokens: mint_msgs };

    let _resp = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let mut i: i32 = 0;
    while i < 5 {
        let query_msg = QueryMsg::AllNftInfo {
            token_id: i.to_string(),
            include_expired: None,
        };
        let query_resp = query(deps.as_ref(), mock_env(), query_msg).unwrap();
        let info: AllNftInfoResponse<TokenExtension> = from_json(&query_resp).unwrap();
        assert_eq!(info.access.owner, owner.to_string());
        i += 1;
    }
}

#[rstest]
#[case::empty_batch("contract_addr", Some(ContractError::EmptyBatch {}), None, 0,5)]
#[case::unauthorized(
    "contract_addr",
    Some(ContractError::Unauthorized {}),
    None,
    3,
    5
)]
#[case::successful(
    "contract_addr",
    None,
    Some("contract_addr".to_string()),
    4,
    5
)]
#[case::too_many_tokens(
    "contract_addr",
    Some(ContractError::Unauthorized {}),
    None,
    6,
    5
)]
fn test_batch_send_nft(
    #[case] contract_addr: &str,
    #[case] expected_error: Option<ContractError>,
    #[case] expected_owner: Option<String>,
    #[case] num_tokens_to_send: u32,
    #[case] num_tokens_to_mint: u32,
) {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let owner = deps.api.addr_make("owner");

    // Setup
    init_setup(&mut deps, env.clone());

    // Mint initial tokens
    let mint_msgs: Vec<MintMsg> = (0..num_tokens_to_mint)
        .map(|i| MintMsg {
            token_id: i.to_string(),
            owner: owner.clone().into(),
            token_uri: None,
            extension: TokenExtension {},
        })
        .collect();

    let mint_msg = ExecuteMsg::BatchMint { tokens: mint_msgs };
    let minter_info = message_info(&Addr::unchecked(MINTER), &[]);
    execute(deps.as_mut(), env.clone(), minter_info, mint_msg).unwrap();

    // Create batch from parameters
    let contract_addr = deps.api.addr_make(contract_addr);
    let batch: Vec<BatchSendMsg> = if num_tokens_to_send == 0 {
        vec![]
    } else {
        (0..num_tokens_to_send)
            .map(|i| BatchSendMsg {
                token_id: i.to_string(),
                contract_addr: AndrAddr::from_string(contract_addr.to_string()),
                msg: Binary::default(),
            })
            .collect()
    };

    // Execute batch send
    let batch_send_msg = ExecuteMsg::BatchSend {
        batch: batch.clone(),
    };
    let unauthorized_addr = deps.api.addr_make("unauthorized");
    let addr = if matches!(expected_error, Some(ContractError::Unauthorized {})) {
        Addr::unchecked(unauthorized_addr)
    } else {
        owner
    };
    let test_info = message_info(&addr, &[]);

    let result = execute(deps.as_mut(), env.clone(), test_info, batch_send_msg);

    match expected_error {
        Some(error) => {
            let err = result.unwrap_err();
            assert_eq!(err, error);
        }
        None => {
            assert!(result.is_ok());
            // Verify final state
            assert_eq!(
                batch.len(),
                num_tokens_to_send as usize,
                "Number of sent tokens doesn't match expected amount"
            );
            for i in 0..num_tokens_to_send {
                let query_msg = QueryMsg::OwnerOf {
                    token_id: i.to_string(),
                    include_expired: None,
                };
                let expected_owner = deps.api.addr_make(&expected_owner.clone().unwrap());
                let query_resp = query(deps.as_ref(), env.clone(), query_msg).unwrap();
                let resp: OwnerOfResponse = from_json(query_resp).unwrap();
                assert_eq!(resp.owner, expected_owner.as_str());
            }
        }
    }
}
