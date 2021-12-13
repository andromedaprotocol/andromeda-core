use crate::contract::{execute, instantiate};
use crate::state::increment_num_tokens;
use andromeda_protocol::{
    address_list::InstantiateMsg as AddressListInstantiateMsg,
    modules::{
        address_list::{ADDRESS_LIST_CONTRACT, REPLY_ADDRESS_LIST},
        ModuleDefinition, Rate,
    },
    receipt::{ExecuteMsg as ReceiptExecuteMsg, Receipt},
    testing::mock_querier::mock_dependencies_custom,
    token::{ExecuteMsg, InstantiateMsg, MintMsg},
};
use cosmwasm_std::{
    attr, coin,
    testing::{mock_env, mock_info},
    to_binary, BankMsg, CosmosMsg, Event, ReplyOn, Response, SubMsg, Uint128, WasmMsg,
};
use cw_storage_plus::Item;

const TOKEN_NAME: &str = "test";
const TOKEN_SYMBOL: &str = "T";
const ADDRESS_LIST_CODE_ID: u64 = 1;
const NUM_TOKENS: Item<u64> = Item::new("numtokens");
// integration testing for initialize, mint, tranfer_aggrement, transferNFT, modules
#[test]
fn test_token_modules() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let env = mock_env();
    let whitelist_moderators = "creator".to_string();
    let tax_fee: Rate = Rate::Percent(1u64);
    let tax_receivers = vec!["tax_recever1".to_string()];
    let royality_fee: Rate = Rate::Percent(1u64);
    let royality_receivers = vec!["royality_recever1".to_string()];
    let modules = vec![
        ModuleDefinition::Whitelist {
            moderators: Some(vec![whitelist_moderators]),
            address: None,
            code_id: Some(ADDRESS_LIST_CODE_ID),
        },
        ModuleDefinition::Taxable {
            rate: tax_fee,
            receivers: tax_receivers,
            description: None,
        },
        ModuleDefinition::Royalties {
            rate: royality_fee,
            receivers: royality_receivers,
            description: None,
        },
        ModuleDefinition::Receipt {
            address: Some("receipt_contract_address".to_string()),
            code_id: Some(2u64), //contract code_id
            moderators: Some(vec!["creator".to_string()]),
        },
    ];
    let msg = InstantiateMsg {
        name: TOKEN_NAME.to_string(),
        symbol: TOKEN_SYMBOL.to_string(),
        modules,
        minter: String::from("creator"),
    };

    let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    let expected_msg = Response::default()
        .add_submessages(vec![SubMsg {
            id: REPLY_ADDRESS_LIST,
            gas_limit: None,
            reply_on: ReplyOn::Always,
            msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some("creator".to_string()),
                code_id: 1u64,
                funds: vec![],
                label: String::from("Address list instantiation"),
                msg: to_binary(&AddressListInstantiateMsg {
                    moderators: vec!["creator".to_string()],
                })
                .unwrap(),
            }),
        }])
        .add_attributes(vec![
            attr("action", "instantiate"),
            attr("name", "test"),
            attr("symbol", "T"),
            attr("minter", "creator"),
        ]);
    assert_eq!(res, expected_msg);

    // set address_list contract address
    ADDRESS_LIST_CONTRACT
        .save(
            deps.as_mut().storage,
            &"addresslist_contract_address1".to_string(),
        )
        .unwrap();
    //test token_mint
    let mint_msg = MintMsg {
        token_id: "token_id1".to_string(),
        owner: "".to_string(),
        description: Some("Test Token".to_string()),
        name: "TestToken".to_string(),
        metadata: None,
        image: None,
        pricing: None,
    };
    let res = execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::Mint(mint_msg.clone()),
    )
    .unwrap();
    let expected = Response::default().add_attributes(vec![
        attr("action", "mint"),
        attr("token_id", mint_msg.token_id),
        attr("owner", info.sender.to_string()),
        attr("name", mint_msg.name),
        attr("symbol", TOKEN_SYMBOL.to_string()),
        attr(
            "pricing",
            match mint_msg.pricing {
                Some(price) => price.to_string(),
                None => String::from("none"),
            },
        ),
        attr(
            "metadata_type",
            match mint_msg.metadata {
                Some(metadata) => metadata.data_type.to_string(),
                None => String::from("unspecified"),
            },
        ),
        attr("publisher", info.sender.to_string()),
        attr("description", String::from("Test Token")),
        attr("image", ""),
    ]);
    assert_eq!(res, expected);
    // test transfer_agreement
    let transfer_agreement_msg = ExecuteMsg::TransferAgreement {
        token_id: "token_id1".to_string(),
        denom: "uusd".to_string(),
        amount: Uint128::from(100u128),
        purchaser: "purchaser1".to_string(),
    };
    let res = execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        transfer_agreement_msg,
    )
    .unwrap();
    assert_eq!(
        res,
        Response::default().add_attributes(vec![
            attr("action", "transfer_agreement"),
            attr("purchaser", "purchaser1"),
            attr("amount", "100uusd"),
            attr("token_id", "token_id1")
        ])
    );

    //test transferNft
    let transfernft_msg = ExecuteMsg::TransferNft {
        recipient: "recipient1".to_string(),
        token_id: "token_id1".to_string(),
    };

    let res = execute(deps.as_mut(), env.clone(), info.clone(), transfernft_msg).unwrap();
    let expected_res = Response::default()
        .add_submessages(vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "creator".to_string(),
                amount: vec![coin(99u128, "uusd".to_string())],
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "tax_recever1".to_string(),
                amount: vec![coin(1u128, "uusd".to_string())], // tax %1 for test
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "royality_recever1".to_string(),
                amount: vec![coin(1u128, "uusd".to_string())], // royality %1 for test
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "receipt_contract_address".to_string(),
                msg: to_binary(&ReceiptExecuteMsg::StoreReceipt {
                    receipt: Receipt {
                        events: vec![
                            Event::new("tax")
                                .add_attributes(vec![attr("payment", "tax_recever1<1uusd")]),
                            Event::new("royalty").add_attributes(vec![
                                attr("deducted", "1uusd"),
                                attr("payment", "royality_recever1<1uusd"),
                            ]),
                            Event::new("agreed_transfer").add_attributes(vec![
                                attr("amount", "100uusd"),
                                attr("purchaser", "purchaser1"),
                            ]),
                        ],
                    },
                })
                .unwrap(),
                funds: vec![],
            })),
        ])
        .add_events(vec![
            Event::new("tax").add_attributes(vec![attr("payment", "tax_recever1<1uusd")]),
            Event::new("royalty").add_attributes(vec![
                attr("deducted", "1uusd"),
                attr("payment", "royality_recever1<1uusd"),
            ]),
            Event::new("agreed_transfer").add_attributes(vec![
                attr("amount", "100uusd"),
                attr("purchaser", "purchaser1"),
            ]),
        ])
        .add_attributes(vec![
            attr("action", "transfer"),
            attr("recipient", "recipient1"),
            attr("token_id", "token_id1"),
            attr("sender", "creator"),
        ]);
    assert_eq!(res, expected_res);
}
// Test for positive test increment for number of tokens
#[test]
fn test_increment_num_tokens() {
    let mut deps = mock_dependencies_custom(&[]);
    let _info = mock_info("creator", &[]);
    let _env = mock_env();
    let res = increment_num_tokens(deps.as_mut().storage).unwrap();
    assert_eq!(res, ());
}
// Increment num tokens with overflow (should panic)
#[test]
#[should_panic]
fn test_increment_num_tokens_error() {
    let mut deps = mock_dependencies_custom(&[]);
    let _info = mock_info("creator", &[]);
    let _env = mock_env();
    NUM_TOKENS.save(deps.as_mut().storage, &u64::MAX).unwrap();
    let _res = increment_num_tokens(deps.as_mut().storage);
}
