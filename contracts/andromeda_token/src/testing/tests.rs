//TODO: FIX MOCK QUERIER
// use crate::contract::{execute, instantiate, query};
// use crate::state::TOKENS;
// use andromeda_protocol::modules::ModuleDefinition;
// use andromeda_protocol::modules::Rate;
// use andromeda_protocol::token::{
//     Approval, ExecuteMsg, InstantiateMsg, MintMsg, NftArchivedResponse, NftMetadataResponse,
//     NftTransferAgreementResponse, QueryMsg, Token, TransferAgreement,
// };
// use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
// use cosmwasm_std::{
//     coin, from_binary, Api, BankMsg, CosmosMsg, Response, StdError, SubMsg, Uint128,
// };
// use cw721::{Expiration, OwnerOfResponse};

// const TOKEN_NAME: &str = "test";
// const TOKEN_SYMBOL: &str = "T";
// const ADDRESS_LIST_CODE_ID: u64 = 1;
// const RECEIPT_CODE_ID: u64 = 2;

// #[test]
// fn test_instantiate() {
//     let mut deps = mock_dependencies(&[]);
//     let info = mock_info("creator", &[]);
//     let whitelist_moderators = "whitelist_moderator1".to_string();
//     let tax_fee: Rate = Rate::Percent(1u64);
//     let tax_receivers = vec!["tax_recever1".to_string()];
//     let royality_fee: Rate = Rate::Percent(1u64);
//     let royality_receivers = vec!["royality_recever1".to_string()];
//     let size_limit = 100u64;
//     let modules = vec![
//         ModuleDefinition::Whitelist {
//             moderators: Some(vec![whitelist_moderators]),
//             address: None,
//             code_id: Some(ADDRESS_LIST_CODE_ID),
//         },
//         ModuleDefinition::Taxable {
//             rate: tax_fee,
//             receivers: tax_receivers,
//             description: None,
//         },
//         ModuleDefinition::Royalties {
//             rate: royality_fee,
//             receivers: royality_receivers,
//             description: None,
//         },
//     ];
//     let msg = InstantiateMsg {
//         name: TOKEN_NAME.to_string(),
//         symbol: TOKEN_SYMBOL.to_string(),
//         modules,
//         minter: String::from("creator"),
//         metadata_limit: Some(size_limit),
//         receipt_code_id: RECEIPT_CODE_ID,
//         address_list_code_id: Some(ADDRESS_LIST_CODE_ID),
//     };

//     let env = mock_env();

//     let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
//     assert_eq!(0, res.messages.len());
// }

// #[test]
// fn test_mint() {
//     let mut deps = mock_dependencies(&[]);
//     let env = mock_env();
//     let info = mock_info("creator", &[]);
//     let token_id = String::default();
//     let creator = "creator".to_string();

//     //Instantiate
//     let whitelist_moderators = vec!["creator".to_string()];
//     let tax_fee: Rate = Rate::Percent(1u64);
//     let tax_receivers = vec!["tax_recever1".to_string()];
//     let royality_fee: Rate = Rate::Percent(1u64);
//     let royality_receivers = vec!["royality_recever1".to_string()];
//     let size_limit = 100u64;
//     let modules = vec![
//         ModuleDefinition::Whitelist {
//             moderators: Some(whitelist_moderators),
//             address: None,
//             code_id: Some(ADDRESS_LIST_CODE_ID),
//         },
//         ModuleDefinition::Taxable {
//             rate: tax_fee,
//             receivers: tax_receivers,
//             description: None,
//         },
//         ModuleDefinition::Royalties {
//             rate: royality_fee,
//             receivers: royality_receivers,
//             description: None,
//         },
//     ];
//     let msg = InstantiateMsg {
//         name: TOKEN_NAME.to_string(),
//         symbol: TOKEN_SYMBOL.to_string(),
//         modules,
//         minter: String::from("creator"),
//         metadata_limit: Some(size_limit),
//         address_list_code_id: Some(ADDRESS_LIST_CODE_ID),
//         receipt_code_id: RECEIPT_CODE_ID,
//     };
//     let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

//     let mint_msg = MintMsg {
//         token_id: token_id.clone(),
//         owner: creator.clone(),
//         description: Some("Test Token".to_string()),
//         name: "TestToken".to_string(),
//         metadata: None,
//     };

//     let msg = ExecuteMsg::Mint(mint_msg);

//     execute(deps.as_mut(), env.clone(), info, msg).unwrap();

//     let query_msg = QueryMsg::OwnerOf { token_id };

//     let query_res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
//     let query_val: OwnerOfResponse = from_binary(&query_res).unwrap();

//     assert_eq!(query_val.owner, creator);
// }

// #[test]
// fn test_transfer() {
//     let mut deps = mock_dependencies(&[]);
//     let env = mock_env();
//     let minter = "minter";
//     let recipient = "recipient";
//     let info = mock_info(minter.clone(), &[]);
//     //Instantiate
//     let whitelist_moderators = vec!["minter".to_string(), "anyone".to_string()];
//     let tax_fee: Rate = Rate::Percent(1u64);
//     let tax_receivers = vec!["tax_recever1".to_string()];
//     let royality_fee: Rate = Rate::Percent(1u64);
//     let royality_receivers = vec!["royality_recever1".to_string()];
//     let size_limit = 100u64;
//     let modules = vec![
//         ModuleDefinition::Whitelist {
//             moderators: Some(whitelist_moderators),
//             address: None,
//             code_id: Some(ADDRESS_LIST_CODE_ID),
//         },
//         ModuleDefinition::Taxable {
//             rate: tax_fee,
//             receivers: tax_receivers,
//             description: None,
//         },
//         ModuleDefinition::Royalties {
//             rate: royality_fee,
//             receivers: royality_receivers,
//             description: None,
//         },
//     ];

//     let msg = InstantiateMsg {
//         name: TOKEN_NAME.to_string(),
//         symbol: TOKEN_SYMBOL.to_string(),
//         modules,
//         minter: String::from("minter"),
//         metadata_limit: Some(size_limit),
//         address_list_code_id: Some(ADDRESS_LIST_CODE_ID),
//         receipt_code_id: RECEIPT_CODE_ID,
//     };
//     let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
//     let token_id = String::default();
//     let msg = ExecuteMsg::TransferNft {
//         recipient: recipient.to_string(),
//         token_id: token_id.clone(),
//     };

//     let token = Token {
//         token_id: token_id.clone(),
//         owner: minter.to_string(),
//         description: None,
//         name: String::default(),
//         approvals: vec![],
//         transfer_agreement: None,
//         metadata: None,
//         archived: false,
//     };

//     TOKENS
//         .save(deps.as_mut().storage, token_id.to_string(), &token)
//         .unwrap();

//     let unauth_info = mock_info("anyone", &[]);

//     let unauth_res =
//         execute(deps.as_mut(), env.clone(), unauth_info.clone(), msg.clone()).unwrap_err();
//     assert_eq!(
//         unauth_res,
//         StdError::generic_err("Address does not have transfer rights for this token")
//     );

//     let notfound_msg = ExecuteMsg::TransferNft {
//         recipient: recipient.to_string(),
//         token_id: String::from("2"),
//     };
//     let notfound_res = execute(
//         deps.as_mut(),
//         env.clone(),
//         info.clone(),
//         notfound_msg.clone(),
//     )
//     .unwrap_err();

//     assert_eq!(
//         notfound_res,
//         StdError::not_found("andromeda_protocol::token::Token")
//     );

//     let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
//     assert_eq!(Response::default(), res);
//     let owner = TOKENS
//         .load(deps.as_ref().storage, token_id.to_string())
//         .unwrap()
//         .owner;
//     assert_eq!(recipient.to_string(), owner);

//     let approval_info = mock_info("minter", &[]);
//     let approval = Approval {
//         spender: approval_info.sender.clone(),
//         expires: cw721::Expiration::Never {},
//     };
//     let approval_token_id = String::from("2");
//     let approval_token = Token {
//         token_id: approval_token_id.clone(),
//         owner: minter.to_string(),
//         description: None,
//         name: String::default(),
//         approvals: vec![approval],
//         transfer_agreement: None,
//         metadata: None,
//         archived: false,
//     };
//     let msg = ExecuteMsg::TransferNft {
//         recipient: recipient.to_string(),
//         token_id: approval_token_id.clone(),
//     };

//     TOKENS
//         .save(
//             deps.as_mut().storage,
//             approval_token_id.to_string(),
//             &approval_token,
//         )
//         .unwrap();

//     let res = execute(
//         deps.as_mut(),
//         env.clone(),
//         approval_info.clone(),
//         msg.clone(),
//     )
//     .unwrap();
//     assert_eq!(Response::default(), res);
//     let owner = TOKENS
//         .load(deps.as_ref().storage, approval_token_id.to_string())
//         .unwrap()
//         .owner;
//     assert_eq!(recipient.to_string(), owner);

//     let approval_info = mock_info("minter", &[]);
//     let approval = Approval {
//         spender: approval_info.sender.clone(),
//         expires: cw721::Expiration::Never {},
//     };
//     let approval_token_id = String::from("2");
//     let approval_token = Token {
//         token_id: approval_token_id.clone(),
//         owner: minter.to_string(),
//         description: None,
//         name: String::default(),
//         approvals: vec![approval],
//         transfer_agreement: None,
//         metadata: None,
//         archived: false,
//     };
//     let msg = ExecuteMsg::TransferNft {
//         recipient: recipient.to_string(),
//         token_id: approval_token_id.clone(),
//     };

//     TOKENS
//         .save(
//             deps.as_mut().storage,
//             approval_token_id.to_string(),
//             &approval_token,
//         )
//         .unwrap();

//     let res = execute(
//         deps.as_mut(),
//         env.clone(),
//         approval_info.clone(),
//         msg.clone(),
//     )
//     .unwrap();
//     assert_eq!(Response::default(), res);
//     let owner = TOKENS
//         .load(deps.as_ref().storage, approval_token_id.to_string())
//         .unwrap()
//         .owner;
//     assert_eq!(recipient.to_string(), owner);
// }

// #[test]
// fn test_agreed_transfer() {
//     let mut deps = mock_dependencies(&[]);
//     let env = mock_env();
//     let minter = "minter";
//     let recipient = "recipient";
//     let info = mock_info(minter.clone(), &[]);
//     let token_id = String::default();
//     //Instantiate
//     let whitelist_moderators = vec!["minter".to_string(), "anyone".to_string()];
//     let tax_fee: Rate = Rate::Percent(1u64);
//     let tax_receivers = vec!["tax_recever1".to_string()];
//     let royality_fee: Rate = Rate::Percent(1u64);
//     let royality_receivers = vec!["royality_recever1".to_string()];
//     let size_limit = 100u64;
//     let modules = vec![
//         ModuleDefinition::Whitelist {
//             moderators: Some(whitelist_moderators),
//             address: None,
//             code_id: Some(ADDRESS_LIST_CODE_ID),
//         },
//         ModuleDefinition::Taxable {
//             rate: tax_fee,
//             receivers: tax_receivers,
//             description: None,
//         },
//         ModuleDefinition::Royalties {
//             rate: royality_fee,
//             receivers: royality_receivers,
//             description: None,
//         },
//     ];

//     let msg = InstantiateMsg {
//         name: TOKEN_NAME.to_string(),
//         symbol: TOKEN_SYMBOL.to_string(),
//         modules,
//         minter: String::from("minter"),
//         metadata_limit: Some(size_limit),
//         address_list_code_id: Some(ADDRESS_LIST_CODE_ID),
//         receipt_code_id: RECEIPT_CODE_ID,
//     };
//     let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
//     let msg = ExecuteMsg::TransferNft {
//         recipient: recipient.to_string(),
//         token_id: token_id.clone(),
//     };
//     let amount = coin(100, "uluna");

//     let token = Token {
//         token_id: token_id.clone(),
//         owner: minter.to_string(),
//         description: None,
//         name: String::default(),
//         approvals: vec![],
//         transfer_agreement: Some(TransferAgreement {
//             purchaser: recipient.to_string(),
//             amount: amount.clone(),
//         }),
//         metadata: None,
//         archived: false,
//     };

//     TOKENS
//         .save(deps.as_mut().storage, token_id.to_string(), &token)
//         .unwrap();

//     let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
//     assert_eq!(
//         res.messages[0],
//         SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
//             to_address: minter.to_string(),
//             amount: vec![coin(100 - 1, "uluna")] // amount - royality
//         }))
//     );
//     assert_eq!(
//         res.messages[1],
//         SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
//             to_address: "tax_recever1".to_string(),
//             amount: vec![coin(amount.amount.u128() * (1u128) / 100u128, "uluna")] // coin.amount / 100 *tax_fee
//         }))
//     );
//     assert_eq!(
//         res.messages[2],
//         SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
//             to_address: "royality_recever1".to_string(),
//             amount: vec![coin(amount.amount.u128() * (1u128) / 100u128, "uluna")] // coin.amount / 100 *tax_fee
//         }))
//     );
// }

// #[test]
// fn test_approve() {
//     let mut deps = mock_dependencies(&[]);
//     let env = mock_env();
//     let sender = "sender";
//     let info = mock_info(sender.clone(), &[]);

//     //Instantiate
//     let whitelist_moderators = vec!["sender".to_string()];
//     let tax_fee: Rate = Rate::Percent(1u64);
//     let tax_receivers = vec!["tax_recever1".to_string()];
//     let royality_fee: Rate = Rate::Percent(1u64);
//     let royality_receivers = vec!["royality_recever1".to_string()];
//     let size_limit = 100u64;
//     let modules = vec![
//         ModuleDefinition::Whitelist {
//             moderators: Some(whitelist_moderators),
//             address: None,
//             code_id: Some(ADDRESS_LIST_CODE_ID),
//         },
//         ModuleDefinition::Taxable {
//             rate: tax_fee,
//             receivers: tax_receivers,
//             description: None,
//         },
//         ModuleDefinition::Royalties {
//             rate: royality_fee,
//             receivers: royality_receivers,
//             description: None,
//         },
//     ];
//     let msg = InstantiateMsg {
//         name: TOKEN_NAME.to_string(),
//         symbol: TOKEN_SYMBOL.to_string(),
//         modules,
//         minter: String::from("minter"),
//         metadata_limit: Some(size_limit),
//         address_list_code_id: Some(ADDRESS_LIST_CODE_ID),
//         receipt_code_id: RECEIPT_CODE_ID,
//     };
//     let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
//     let token_id = String::default();
//     let approvee = "aprovee";

//     let msg = ExecuteMsg::Approve {
//         spender: approvee.to_string(),
//         expires: None,
//         token_id: String::default(),
//     };

//     let token = Token {
//         token_id: token_id.clone(),
//         description: None,
//         name: String::default(),
//         approvals: vec![],
//         owner: sender.to_string(),
//         transfer_agreement: None,
//         metadata: None,
//         archived: false,
//     };

//     TOKENS
//         .save(deps.as_mut().storage, token_id.to_string(), &token)
//         .unwrap();

//     execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
//     let token = TOKENS
//         .load(deps.as_mut().storage, token_id.to_string())
//         .unwrap();

//     assert_eq!(1, token.approvals.len());
//     assert_eq!(approvee.clone(), token.approvals[0].spender.to_string());
// }

// #[test]
// fn test_revoke() {
//     let mut deps = mock_dependencies(&[]);
//     let env = mock_env();
//     let sender = "sender";
//     let info = mock_info(sender.clone(), &[]);

//     //Instantiate
//     let whitelist_moderators = vec!["sender".to_string()];
//     let tax_fee: Rate = Rate::Percent(1u64);
//     let tax_receivers = vec!["tax_recever1".to_string()];
//     let royality_fee: Rate = Rate::Percent(1u64);
//     let royality_receivers = vec!["royality_recever1".to_string()];
//     let size_limit = 100u64;
//     let modules = vec![
//         ModuleDefinition::Whitelist {
//             moderators: Some(whitelist_moderators),
//             address: None,
//             code_id: Some(ADDRESS_LIST_CODE_ID),
//         },
//         ModuleDefinition::Taxable {
//             rate: tax_fee,
//             receivers: tax_receivers,
//             description: None,
//         },
//         ModuleDefinition::Royalties {
//             rate: royality_fee,
//             receivers: royality_receivers,
//             description: None,
//         },
//     ];
//     let msg = InstantiateMsg {
//         name: TOKEN_NAME.to_string(),
//         symbol: TOKEN_SYMBOL.to_string(),
//         modules,
//         minter: String::from("minter"),
//         metadata_limit: Some(size_limit),
//         address_list_code_id: Some(ADDRESS_LIST_CODE_ID),
//         receipt_code_id: RECEIPT_CODE_ID,
//     };
//     let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

//     let token_id = String::default();
//     let approvee = "aprovee";
//     let approval = Approval {
//         expires: Expiration::Never {},
//         spender: deps.api.addr_validate(approvee.clone()).unwrap(),
//     };

//     let msg = ExecuteMsg::Revoke {
//         spender: approvee.to_string(),
//         token_id: String::default(),
//     };

//     let token = Token {
//         token_id: token_id.clone(),
//         description: None,
//         name: String::default(),
//         approvals: vec![approval],
//         owner: sender.to_string(),
//         transfer_agreement: None,
//         metadata: None,
//         archived: false,
//     };

//     TOKENS
//         .save(deps.as_mut().storage, token_id.to_string(), &token)
//         .unwrap();

//     execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
//     let token = TOKENS
//         .load(deps.as_mut().storage, token_id.to_string())
//         .unwrap();

//     assert_eq!(0, token.approvals.len());
// }

// #[test]
// fn test_approve_all() {
//     let mut deps = mock_dependencies(&[]);
//     let env = mock_env();
//     let minter = "minter";
//     let info = mock_info(minter.clone(), &[]);
//     let token_id = String::default();
//     let operator = "operator";

//     //Instantiate
//     let whitelist_moderators = vec![minter.to_string(), operator.to_string()];
//     let tax_fee: Rate = Rate::Percent(1u64);
//     let tax_receivers = vec!["tax_recever1".to_string()];
//     let royality_fee: Rate = Rate::Percent(1u64);
//     let royality_receivers = vec!["royality_recever1".to_string()];
//     let size_limit = 100u64;
//     let modules = vec![
//         ModuleDefinition::Whitelist {
//             moderators: Some(whitelist_moderators),
//             address: None,
//             code_id: Some(ADDRESS_LIST_CODE_ID),
//         },
//         ModuleDefinition::Taxable {
//             rate: tax_fee,
//             receivers: tax_receivers,
//             description: None,
//         },
//         ModuleDefinition::Royalties {
//             rate: royality_fee,
//             receivers: royality_receivers,
//             description: None,
//         },
//     ];
//     let msg = InstantiateMsg {
//         name: TOKEN_NAME.to_string(),
//         symbol: TOKEN_SYMBOL.to_string(),
//         modules,
//         minter: String::from("minter"),
//         metadata_limit: Some(size_limit),
//         address_list_code_id: Some(ADDRESS_LIST_CODE_ID),
//         receipt_code_id: RECEIPT_CODE_ID,
//     };
//     let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
//     let operator_info = mock_info(operator.clone(), &[]);

//     let mint_msg = ExecuteMsg::Mint(MintMsg {
//         token_id: token_id.clone(),
//         owner: minter.to_string(),
//         description: None,
//         name: "Some Token".to_string(),
//         metadata: None,
//     });
//     execute(deps.as_mut(), env.clone(), info.clone(), mint_msg).unwrap();

//     let transfer_msg = ExecuteMsg::TransferNft {
//         recipient: operator.to_string(),
//         token_id: token_id.clone(),
//     };
//     let err = execute(
//         deps.as_mut(),
//         env.clone(),
//         operator_info.clone(),
//         transfer_msg,
//     )
//     .unwrap_err();

//     assert_eq!(
//         err,
//         StdError::generic_err("Address does not have transfer rights for this token"),
//     );

//     let approve_all_msg = ExecuteMsg::ApproveAll {
//         operator: operator.to_string(),
//         expires: None,
//     };
//     execute(deps.as_mut(), env.clone(), info.clone(), approve_all_msg).unwrap();

//     let transfer_msg = ExecuteMsg::TransferNft {
//         recipient: operator.to_string(),
//         token_id: token_id.clone(),
//     };
//     execute(
//         deps.as_mut(),
//         env.clone(),
//         operator_info.clone(),
//         transfer_msg,
//     )
//     .unwrap();

//     let token = TOKENS
//         .load(deps.as_ref().storage, token_id.to_string())
//         .unwrap();

//     assert_eq!(token.owner, operator.to_string());
// }

// #[test]
// fn test_revoke_all() {
//     let mut deps = mock_dependencies(&[]);
//     let env = mock_env();
//     let minter = "minter";
//     let info = mock_info(minter.clone(), &[]);
//     let token_id = String::default();
//     let operator = "operator";
//     let operator_info = mock_info(operator.clone(), &[]);
//     //Instantiate
//     let whitelist_moderators = vec![minter.to_string(), operator.to_string()];
//     let tax_fee: Rate = Rate::Percent(1u64);
//     let tax_receivers = vec!["tax_recever1".to_string()];
//     let royality_fee: Rate = Rate::Percent(1u64);
//     let royality_receivers = vec!["royality_recever1".to_string()];
//     let size_limit = 100u64;
//     let modules = vec![
//         ModuleDefinition::Whitelist {
//             moderators: Some(whitelist_moderators),
//             address: None,
//             code_id: Some(ADDRESS_LIST_CODE_ID),
//         },
//         ModuleDefinition::Taxable {
//             rate: tax_fee,
//             receivers: tax_receivers,
//             description: None,
//         },
//         ModuleDefinition::Royalties {
//             rate: royality_fee,
//             receivers: royality_receivers,
//             description: None,
//         },
//     ];

//     let msg = InstantiateMsg {
//         name: TOKEN_NAME.to_string(),
//         symbol: TOKEN_SYMBOL.to_string(),
//         modules,
//         minter: String::from("minter"),
//         metadata_limit: Some(size_limit),
//         address_list_code_id: Some(ADDRESS_LIST_CODE_ID),
//         receipt_code_id: RECEIPT_CODE_ID,
//     };
//     let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
//     let mint_msg = ExecuteMsg::Mint(MintMsg {
//         token_id: token_id.clone(),
//         owner: minter.to_string(),
//         description: None,
//         name: "Some Token".to_string(),
//         metadata: None,
//     });
//     execute(deps.as_mut(), env.clone(), info.clone(), mint_msg).unwrap();

//     let approve_all_msg = ExecuteMsg::ApproveAll {
//         operator: operator.to_string(),
//         expires: None,
//     };
//     execute(deps.as_mut(), env.clone(), info.clone(), approve_all_msg).unwrap();

//     let transfer_msg = ExecuteMsg::TransferNft {
//         recipient: minter.to_string(),
//         token_id: token_id.clone(),
//     };
//     execute(
//         deps.as_mut(),
//         env.clone(),
//         operator_info.clone(),
//         transfer_msg,
//     )
//     .unwrap();

//     let revoke_msg = ExecuteMsg::RevokeAll {
//         operator: operator.to_string(),
//     };
//     execute(deps.as_mut(), env.clone(), info.clone(), revoke_msg).unwrap();

//     let transfer_msg = ExecuteMsg::TransferNft {
//         recipient: minter.to_string(),
//         token_id: token_id.clone(),
//     };
//     let err = execute(
//         deps.as_mut(),
//         env.clone(),
//         operator_info.clone(),
//         transfer_msg,
//     )
//     .unwrap_err();

//     assert_eq!(
//         err,
//         StdError::generic_err("Address does not have transfer rights for this token"),
//     );
// }

// #[test]
// fn test_transfer_agreement() {
//     let mut deps = mock_dependencies(&[]);
//     let env = mock_env();
//     let minter = "minter";
//     let purchaser = "purchaser";
//     let info = mock_info(minter.clone(), &[]);
//     let token_id = String::default();
//     let denom = "uluna";
//     let amount = 100 as u128;
//     //Instantiate
//     let whitelist_moderators = vec![minter.to_string(), purchaser.to_string()];
//     let tax_fee: Rate = Rate::Percent(1u64);
//     let tax_receivers = vec!["tax_recever1".to_string()];
//     let royality_fee: Rate = Rate::Percent(1u64);
//     let royality_receivers = vec!["royality_recever1".to_string()];
//     let size_limit = 100u64;
//     let modules = vec![
//         ModuleDefinition::Whitelist {
//             moderators: Some(whitelist_moderators),
//             address: None,
//             code_id: Some(ADDRESS_LIST_CODE_ID),
//         },
//         ModuleDefinition::Taxable {
//             rate: tax_fee,
//             receivers: tax_receivers,
//             description: None,
//         },
//         ModuleDefinition::Royalties {
//             rate: royality_fee,
//             receivers: royality_receivers,
//             description: None,
//         },
//     ];
//     let msg = InstantiateMsg {
//         name: TOKEN_NAME.to_string(),
//         symbol: TOKEN_SYMBOL.to_string(),
//         modules,
//         minter: minter.to_string(),
//         metadata_limit: Some(size_limit),
//         address_list_code_id: Some(ADDRESS_LIST_CODE_ID),
//         receipt_code_id: RECEIPT_CODE_ID,
//     };
//     let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

//     let mint_msg = ExecuteMsg::Mint(MintMsg {
//         token_id: token_id.clone(),
//         owner: minter.to_string(),
//         description: None,
//         name: "Some Token".to_string(),
//         metadata: None,
//     });
//     execute(deps.as_mut(), env.clone(), info.clone(), mint_msg).unwrap();

//     let transfer_agreement_msg = ExecuteMsg::TransferAgreement {
//         token_id: token_id.clone(),
//         denom: denom.to_string(),
//         amount: Uint128::from(amount),
//         purchaser: purchaser.to_string(),
//     };
//     execute(
//         deps.as_mut(),
//         env.clone(),
//         info.clone(),
//         transfer_agreement_msg,
//     )
//     .unwrap();

//     let agreement_query = QueryMsg::NftTransferAgreementInfo {
//         token_id: token_id.clone(),
//     };
//     let res = query(deps.as_ref(), env.clone(), agreement_query).unwrap();
//     let agreement_res: NftTransferAgreementResponse = from_binary(&res).unwrap();
//     let agreement = agreement_res.agreement.unwrap();

//     assert_eq!(agreement.purchaser, purchaser.clone());
//     assert_eq!(agreement.amount, coin(amount, denom));

//     let purchaser_info = mock_info(purchaser.clone(), &[]);
//     let transfer_msg = ExecuteMsg::TransferNft {
//         token_id: token_id.clone(),
//         recipient: purchaser.to_string(),
//     };
//     execute(
//         deps.as_mut(),
//         env.clone(),
//         purchaser_info.clone(),
//         transfer_msg,
//     )
//     .unwrap();
// }

// #[test]
// fn test_metadata() {
//     let mut deps = mock_dependencies(&[]);
//     let env = mock_env();
//     let minter = "minter";
//     let info = mock_info(minter.clone(), &[]);
//     let token_id = "1";
//     //Instantiate
//     let whitelist_moderators = vec!["minter".to_string(), "anyone".to_string()];
//     let tax_fee: Rate = Rate::Percent(1u64);
//     let tax_receivers = vec!["tax_recever1".to_string()];
//     let royality_fee: Rate = Rate::Percent(1u64);
//     let royality_receivers = vec!["royality_recever1".to_string()];
//     let size_limit = 100u64;
//     let modules = vec![
//         ModuleDefinition::Whitelist {
//             moderators: Some(whitelist_moderators),
//             address: None,
//             code_id: Some(ADDRESS_LIST_CODE_ID),
//         },
//         ModuleDefinition::Taxable {
//             rate: tax_fee,
//             receivers: tax_receivers,
//             description: None,
//         },
//         ModuleDefinition::Royalties {
//             rate: royality_fee,
//             receivers: royality_receivers,
//             description: None,
//         },
//     ];

//     let instantiate_message = InstantiateMsg {
//         name: "Token".to_string(),
//         symbol: "T".to_string(),
//         minter: minter.to_string(),
//         modules: modules,
//         metadata_limit: Some(size_limit),
//         address_list_code_id: Some(ADDRESS_LIST_CODE_ID),
//         receipt_code_id: RECEIPT_CODE_ID,
//     };

//     instantiate(
//         deps.as_mut(),
//         env.clone(),
//         info.clone(),
//         instantiate_message,
//     )
//     .unwrap();
//     let metadata = "really long metadata message, too long for the storage".to_string();
//     let mint_msg = ExecuteMsg::Mint(MintMsg {
//         token_id: token_id.to_string(),
//         owner: minter.to_string(),
//         name: "test token".to_string(),
//         description: None,
//         metadata: Some(metadata.clone()),
//     });
//     let res = execute(deps.as_mut(), env.clone(), info.clone(), mint_msg).unwrap_err();
//     assert_eq!(
//         res,
//         StdError::generic_err("Metadata length must be less than or equal to 4")
//     );
//     let metadata = "s".to_string();
//     let mint_msg = ExecuteMsg::Mint(MintMsg {
//         token_id: token_id.to_string(),
//         owner: minter.to_string(),
//         name: "test token".to_string(),
//         description: None,
//         metadata: Some(metadata.clone()),
//     });

//     let res = execute(deps.as_mut(), env.clone(), info.clone(), mint_msg).unwrap();

//     assert_eq!(res, Response::default());

//     let query_msg = QueryMsg::NftMetadata {
//         token_id: token_id.to_string(),
//     };

//     let query_res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
//     let query_val: NftMetadataResponse = from_binary(&query_res).unwrap();

//     assert_eq!(query_val.metadata, Some(metadata.clone()))
// }

// #[test]
// fn test_execute_burn() {
//     let mut deps = mock_dependencies(&[]);
//     let env = mock_env();
//     let minter = "minter";
//     let info = mock_info(minter.clone(), &[]);
//     let token_id = "1";
//     //Instantiate
//     let whitelist_moderators = vec!["minter".to_string(), "anyone".to_string()];
//     let tax_fee: Rate = Rate::Percent(1u64);
//     let tax_receivers = vec!["tax_recever1".to_string()];
//     let royality_fee: Rate = Rate::Percent(1u64);
//     let royality_receivers = vec!["royality_recever1".to_string()];
//     let size_limit = 100u64;
//     let modules = vec![
//         ModuleDefinition::Whitelist {
//             moderators: Some(whitelist_moderators),
//             address: None,
//             code_id: Some(ADDRESS_LIST_CODE_ID),
//         },
//         ModuleDefinition::Taxable {
//             rate: tax_fee,
//             receivers: tax_receivers,
//             description: None,
//         },
//         ModuleDefinition::Royalties {
//             rate: royality_fee,
//             receivers: royality_receivers,
//             description: None,
//         },
//     ];
//     let msg = InstantiateMsg {
//         name: TOKEN_NAME.to_string(),
//         symbol: TOKEN_SYMBOL.to_string(),
//         modules,
//         minter: String::from("minter"),
//         metadata_limit: Some(size_limit),
//         address_list_code_id: Some(ADDRESS_LIST_CODE_ID),
//         receipt_code_id: RECEIPT_CODE_ID,
//     };
//     let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
//     let mint_msg = MintMsg {
//         token_id: token_id.to_string(),
//         owner: minter.to_string(),
//         description: Some("Test Token".to_string()),
//         name: "TestToken".to_string(),
//         metadata: None,
//     };

//     let msg = ExecuteMsg::Mint(mint_msg);

//     execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

//     let unauth_info = mock_info("anyone", &[]);
//     let burn_msg = ExecuteMsg::Burn {
//         token_id: token_id.to_string(),
//     };

//     let resp = execute(deps.as_mut(), env.clone(), unauth_info, burn_msg.clone()).unwrap_err();

//     assert_eq!(
//         resp,
//         StdError::generic_err("Cannot burn a token you do not own")
//     );

//     execute(deps.as_mut(), env.clone(), info.clone(), burn_msg.clone()).unwrap();

//     let query_msg = QueryMsg::OwnerOf {
//         token_id: token_id.to_string(),
//     };

//     let query_res = query(deps.as_ref(), env.clone(), query_msg).unwrap_err();

//     assert_eq!(
//         query_res,
//         StdError::not_found("andromeda_protocol::token::Token")
//     )
// }

// #[test]
// fn test_execute_archive() {
//     let mut deps = mock_dependencies(&[]);
//     let env = mock_env();
//     let minter = "minter";
//     let info = mock_info(minter.clone(), &[]);
//     let token_id = "1";
//     //Instantiate
//     let whitelist_moderators = vec!["minter".to_string(), "anyone".to_string()];
//     let tax_fee: Rate = Rate::Percent(1u64);
//     let tax_receivers = vec!["tax_recever1".to_string()];
//     let royality_fee: Rate = Rate::Percent(1u64);
//     let royality_receivers = vec!["royality_recever1".to_string()];
//     let size_limit = 100u64;
//     let modules = vec![
//         ModuleDefinition::Whitelist {
//             moderators: Some(whitelist_moderators),
//             address: None,
//             code_id: Some(ADDRESS_LIST_CODE_ID),
//         },
//         ModuleDefinition::Taxable {
//             rate: tax_fee,
//             receivers: tax_receivers,
//             description: None,
//         },
//         ModuleDefinition::Royalties {
//             rate: royality_fee,
//             receivers: royality_receivers,
//             description: None,
//         },
//     ];
//     let msg = InstantiateMsg {
//         name: TOKEN_NAME.to_string(),
//         symbol: TOKEN_SYMBOL.to_string(),
//         modules,
//         minter: String::from("minter"),
//         metadata_limit: Some(size_limit),
//         address_list_code_id: Some(ADDRESS_LIST_CODE_ID),
//         receipt_code_id: RECEIPT_CODE_ID,
//     };
//     let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
//     let mint_msg = MintMsg {
//         token_id: token_id.to_string(),
//         owner: minter.to_string(),
//         description: Some("Test Token".to_string()),
//         name: "TestToken".to_string(),
//         metadata: None,
//     };

//     let msg = ExecuteMsg::Mint(mint_msg);

//     execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

//     let unauth_info = mock_info("anyone", &[]);
//     let archive_msg = ExecuteMsg::Archive {
//         token_id: token_id.to_string(),
//     };

//     let resp = execute(deps.as_mut(), env.clone(), unauth_info, archive_msg.clone()).unwrap_err();

//     assert_eq!(
//         resp,
//         StdError::generic_err("Cannot archive a token you do not own")
//     );

//     execute(
//         deps.as_mut(),
//         env.clone(),
//         info.clone(),
//         archive_msg.clone(),
//     )
//     .unwrap();

//     let archived_resp = execute(
//         deps.as_mut(),
//         env.clone(),
//         info.clone(),
//         archive_msg.clone(),
//     )
//     .unwrap_err();
//     assert_eq!(
//         archived_resp,
//         StdError::generic_err("This token is archived and cannot be changed in any way.")
//     );

//     let query_msg = QueryMsg::NftArchiveStatus {
//         token_id: token_id.to_string(),
//     };

//     let query_res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
//     let query_val: NftArchivedResponse = from_binary(&query_res).unwrap();
//     assert!(query_val.archived)
// }
