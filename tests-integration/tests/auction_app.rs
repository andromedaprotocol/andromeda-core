#![cfg(not(target_arch = "wasm32"))]

use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, mock_claim_ownership_msg, MockAppContract};
use andromeda_auction::mock::{
    mock_andromeda_auction, mock_auction_instantiate_msg, mock_authorize_token_address,
    mock_claim_auction, mock_get_auction_ids, mock_get_auction_state, mock_get_bids,
    mock_place_bid, mock_set_permission, mock_start_auction, MockAuction,
};
use andromeda_cw20::mock::{
    mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_cw20_send, mock_get_cw20_balance,
    mock_minter, MockCW20,
};
use andromeda_cw721::mock::{
    mock_andromeda_cw721, mock_cw721_instantiate_msg, mock_cw721_owner_of, mock_quick_mint_msg,
    mock_send_nft, MockCW721,
};

use andromeda_finance::splitter::AddressPercent;
use andromeda_modules::rates::{PercentRate, Rate, RateInfo};
use andromeda_non_fungible_tokens::auction::{
    AuctionIdsResponse, AuctionStateResponse, BidsResponse, Cw20HookMsg,
};
use andromeda_rates::mock::{mock_andromeda_rates, mock_rates_instantiate_msg};
use andromeda_splitter::mock::{
    mock_andromeda_splitter, mock_splitter_instantiate_msg, mock_splitter_send_msg,
};
use andromeda_std::{
    ado_base::{permissioning::Permission, Module},
    amp::{AndrAddr, Recipient},
    common::{expiration::MILLISECONDS_TO_NANOSECONDS_RATIO, Milliseconds},
    error::ContractError,
};
use andromeda_testing::{
    mock::mock_app, mock_builder::MockAndromedaBuilder, mock_contract::MockContract,
};
use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Decimal, Timestamp, Uint128};
use cw20::{BalanceResponse, Cw20Coin};
use cw721::OwnerOfResponse;
use cw_multi_test::Executor;

#[test]
fn test_auction_app_modules() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![]),
            ("buyer_one", vec![coin(1000, "uandr")]),
            ("buyer_two", vec![coin(1000, "uandr")]),
            ("recipient_one", vec![]),
            ("recipient_two", vec![]),
        ])
        .with_contracts(vec![
            ("cw721", mock_andromeda_cw721()),
            ("auction", mock_andromeda_auction()),
            ("app-contract", mock_andromeda_app()),
            ("rates", mock_andromeda_rates()),
            ("splitter", mock_andromeda_splitter()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let buyer_one = andr.get_wallet("buyer_one");
    let buyer_two = andr.get_wallet("buyer_two");
    let recipient_one = andr.get_wallet("recipient_one");
    let recipient_two = andr.get_wallet("recipient_two");

    // Generate App Components
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        owner.to_string(),
        None,
        andr.kernel.addr().to_string(),
        None,
    );
    let cw721_component = AppComponent::new(
        "cw721".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    let rates_init_msg = mock_rates_instantiate_msg(
        vec![RateInfo {
            is_additive: false,
            description: None,
            rate: Rate::Percent(PercentRate {
                percent: Decimal::from_ratio(1u32, 2u32),
            }),
            recipients: vec![
                Recipient::from_string("./splitter").with_msg(mock_splitter_send_msg())
            ],
        }],
        andr.kernel.addr(),
        None,
    );
    let rates_component =
        AppComponent::new("rates", "rates", to_json_binary(&rates_init_msg).unwrap());

    let splitter_init_msg = mock_splitter_instantiate_msg(
        vec![
            AddressPercent::new(
                Recipient::from_string(format!("{recipient_one}")),
                Decimal::from_ratio(1u8, 2u8),
            ),
            AddressPercent::new(
                Recipient::from_string(format!("{recipient_two}")),
                Decimal::from_ratio(1u8, 2u8),
            ),
        ],
        andr.kernel.addr(),
        None,
        None,
    );
    let splitter_component = AppComponent::new(
        "splitter",
        "splitter",
        to_json_binary(&splitter_init_msg).unwrap(),
    );

    let auction_init_msg = mock_auction_instantiate_msg(
        Some(vec![Module::new("rates", "./rates", false)]),
        andr.kernel.addr().to_string(),
        None,
        Some(vec![AndrAddr::from_string(format!(
            "./{}",
            cw721_component.name
        ))]),
        None,
    );
    let auction_component = AppComponent::new(
        "auction".to_string(),
        "auction".to_string(),
        to_json_binary(&auction_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![
        cw721_component.clone(),
        auction_component.clone(),
        rates_component,
        splitter_component,
    ];
    let app = MockAppContract::instantiate(
        andr.get_code_id(&mut router, "app-contract"),
        owner,
        &mut router,
        "Auction App",
        app_components,
        andr.kernel.addr(),
        Some(owner.to_string()),
    );

    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(app.addr().clone()),
            &mock_claim_ownership_msg(None),
            &[],
        )
        .unwrap();

    // Mint Tokens
    let cw721: MockCW721 = app.query_ado_by_component_name(&router, cw721_component.name);
    cw721
        .execute_quick_mint(&mut router, owner.clone(), 1, owner.to_string())
        .unwrap();

    // Send Token to Auction
    let auction: MockAuction = app.query_ado_by_component_name(&router, auction_component.name);
    let start_time = Milliseconds::from_nanos(router.block_info().time.nanos())
        .plus_milliseconds(Milliseconds(100));
    let receive_msg = mock_start_auction(
        Some(start_time),
        start_time.plus_milliseconds(Milliseconds(1000)),
        "uandr".to_string(),
        false,
        None,
        None,
        None,
    );
    cw721
        .execute_send_nft(
            &mut router,
            owner.clone(),
            auction.addr(),
            "0",
            &receive_msg,
        )
        .unwrap();

    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: start_time.into(),
        chain_id: router.block_info().chain_id,
    });

    // Query Auction State
    let auction_ids: Vec<Uint128> =
        auction.query_auction_ids(&mut router, "0".to_string(), cw721.addr().to_string());

    assert_eq!(auction_ids.len(), 1);

    let auction_id = auction_ids.first().unwrap();
    let auction_state = auction.query_auction_state(&mut router, *auction_id);

    assert_eq!(auction_state.coin_denom, "uandr".to_string());
    assert_eq!(auction_state.owner, owner.to_string());

    // Place Bid One
    auction.execute_place_bid(
        &mut router,
        buyer_one.clone(),
        "0".to_string(),
        cw721.addr().to_string(),
        &[coin(50, "uandr")],
    );

    // Check Bid Status One
    let bids = auction.query_bids(&mut router, *auction_id);
    assert_eq!(bids.len(), 1);

    let bid = bids.first().unwrap();
    assert_eq!(bid.bidder, buyer_one.to_string());
    assert_eq!(bid.amount, Uint128::from(50u128));

    auction.execute_place_bid(
        &mut router,
        buyer_two.clone(),
        "0".to_string(),
        cw721.addr().to_string(),
        &[coin(100, "uandr")],
    );

    // Check Bid Status One
    let bids = auction.query_bids(&mut router, *auction_id);
    assert_eq!(bids.len(), 2);

    let bid_two = bids.get(1).unwrap();
    assert_eq!(bid_two.bidder, buyer_two.to_string());
    assert_eq!(bid_two.amount, Uint128::from(100u128));

    // End Auction
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: start_time.plus_milliseconds(Milliseconds(1000)).into(),
        chain_id: router.block_info().chain_id,
    });
    auction
        .execute_claim_auction(
            &mut router,
            buyer_two.clone(),
            "0".to_string(),
            cw721.addr().to_string(),
        )
        .unwrap();

    // Check Final State
    let token_owner = cw721.query_owner_of(&router, "0");
    assert_eq!(token_owner, buyer_two);
    let owner_balance = router.wrap().query_balance(owner, "uandr").unwrap();
    assert_eq!(owner_balance.amount, Uint128::from(50u128));
    let recipient_one_balance = router.wrap().query_balance(recipient_one, "uandr").unwrap();
    assert_eq!(recipient_one_balance.amount, Uint128::from(25u128));
    let recipient_two_balance = router.wrap().query_balance(recipient_two, "uandr").unwrap();
    assert_eq!(recipient_two_balance.amount, Uint128::from(25u128));
}

#[test]
fn test_auction_app_recipient() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![]),
            ("buyer_one", vec![coin(1000, "uandr")]),
            ("buyer_two", vec![coin(1000, "uandr")]),
            ("recipient_one", vec![]),
            ("recipient_two", vec![]),
        ])
        .with_contracts(vec![
            ("cw721", mock_andromeda_cw721()),
            ("auction", mock_andromeda_auction()),
            ("app-contract", mock_andromeda_app()),
            ("splitter", mock_andromeda_splitter()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let buyer_one = andr.get_wallet("buyer_one");
    let buyer_two = andr.get_wallet("buyer_two");
    let recipient_one = andr.get_wallet("recipient_one");
    let recipient_two = andr.get_wallet("recipient_two");

    // Generate App Components
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        owner.to_string(),
        None,
        andr.kernel.addr().to_string(),
        None,
    );
    let cw721_component = AppComponent::new(
        "cw721".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    let splitter_init_msg = mock_splitter_instantiate_msg(
        vec![
            AddressPercent::new(
                Recipient::from_string(format!("{recipient_one}")),
                Decimal::from_ratio(1u8, 2u8),
            ),
            AddressPercent::new(
                Recipient::from_string(format!("{recipient_two}")),
                Decimal::from_ratio(1u8, 2u8),
            ),
        ],
        andr.kernel.addr(),
        None,
        None,
    );
    let splitter_component = AppComponent::new(
        "splitter",
        "splitter",
        to_json_binary(&splitter_init_msg).unwrap(),
    );

    let auction_init_msg =
        mock_auction_instantiate_msg(None, andr.kernel.addr().to_string(), None, None, None);
    let auction_component = AppComponent::new(
        "auction".to_string(),
        "auction".to_string(),
        to_json_binary(&auction_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![
        cw721_component.clone(),
        auction_component.clone(),
        splitter_component,
    ];
    let app = MockAppContract::instantiate(
        andr.get_code_id(&mut router, "app-contract"),
        owner,
        &mut router,
        "Auction App",
        app_components,
        andr.kernel.addr(),
        Some(owner.to_string()),
    );

    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(app.addr().clone()),
            &mock_claim_ownership_msg(None),
            &[],
        )
        .unwrap();

    // Mint Tokens
    let cw721: MockCW721 = app.query_ado_by_component_name(&router, cw721_component.name);
    cw721
        .execute_quick_mint(&mut router, owner.clone(), 1, owner.to_string())
        .unwrap();

    // Send Token to Auction
    let auction: MockAuction = app.query_ado_by_component_name(&router, auction_component.name);
    let start_time = Milliseconds::from_nanos(router.block_info().time.nanos())
        .plus_milliseconds(Milliseconds(100));
    let receive_msg = mock_start_auction(
        Some(start_time),
        start_time.plus_milliseconds(Milliseconds(1000)),
        "uandr".to_string(),
        false,
        None,
        None,
        Some(Recipient::from_string("./splitter").with_msg(mock_splitter_send_msg())),
    );
    cw721
        .execute_send_nft(
            &mut router,
            owner.clone(),
            auction.addr(),
            "0",
            &receive_msg,
        )
        .unwrap();

    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: start_time.into(),
        chain_id: router.block_info().chain_id,
    });

    // Query Auction State
    let auction_ids: Vec<Uint128> =
        auction.query_auction_ids(&mut router, "0".to_string(), cw721.addr().to_string());

    assert_eq!(auction_ids.len(), 1);

    let auction_id = auction_ids.first().unwrap();
    let auction_state = auction.query_auction_state(&mut router, *auction_id);

    assert_eq!(auction_state.coin_denom, "uandr".to_string());
    assert_eq!(auction_state.owner, owner.to_string());

    // Place Bid One
    auction.execute_place_bid(
        &mut router,
        buyer_one.clone(),
        "0".to_string(),
        cw721.addr().to_string(),
        &[coin(50, "uandr")],
    );

    // Check Bid Status One
    let bids = auction.query_bids(&mut router, *auction_id);
    assert_eq!(bids.len(), 1);

    let bid = bids.first().unwrap();
    assert_eq!(bid.bidder, buyer_one.to_string());
    assert_eq!(bid.amount, Uint128::from(50u128));

    auction.execute_place_bid(
        &mut router,
        buyer_two.clone(),
        "0".to_string(),
        cw721.addr().to_string(),
        &[coin(100, "uandr")],
    );

    // Check Bid Status One
    let bids = auction.query_bids(&mut router, *auction_id);
    assert_eq!(bids.len(), 2);

    let bid_two = bids.get(1).unwrap();
    assert_eq!(bid_two.bidder, buyer_two.to_string());
    assert_eq!(bid_two.amount, Uint128::from(100u128));

    // End Auction
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: start_time.plus_milliseconds(Milliseconds(1000)).into(),
        chain_id: router.block_info().chain_id,
    });
    auction
        .execute_claim_auction(
            &mut router,
            buyer_two.clone(),
            "0".to_string(),
            cw721.addr().to_string(),
        )
        .unwrap();

    // Check Final State
    let token_owner = cw721.query_owner_of(&router, "0");
    assert_eq!(token_owner, buyer_two);
    let owner_balance = router.wrap().query_balance(owner, "uandr").unwrap();
    assert_eq!(owner_balance.amount, Uint128::zero());
    let recipient_one_balance = router.wrap().query_balance(recipient_one, "uandr").unwrap();
    assert_eq!(recipient_one_balance.amount, Uint128::from(50u128));
    let recipient_two_balance = router.wrap().query_balance(recipient_two, "uandr").unwrap();
    assert_eq!(recipient_two_balance.amount, Uint128::from(50u128));
}

#[test]
fn test_auction_app_cw20() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![]),
            ("buyer_one", vec![coin(1000, "uandr")]),
            ("buyer_two", vec![coin(1000, "uandr")]),
            ("recipient_one", vec![]),
            ("recipient_two", vec![]),
        ])
        .with_contracts(vec![
            ("cw721", mock_andromeda_cw721()),
            ("cw20", mock_andromeda_cw20()),
            ("auction", mock_andromeda_auction()),
            ("app-contract", mock_andromeda_app()),
            ("rates", mock_andromeda_rates()),
            ("splitter", mock_andromeda_splitter()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let buyer_one = andr.get_wallet("buyer_one");
    let buyer_two = andr.get_wallet("buyer_two");

    // Generate App Components
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        owner.to_string(),
        None,
        andr.kernel.addr().to_string(),
        None,
    );
    let cw721_component = AppComponent::new(
        "cw721".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    let buyer_one_original_balance = Uint128::new(1_000);
    let buyer_two_original_balance = Uint128::new(2_000);
    let owner_original_balance = Uint128::new(10_000);
    let initial_balances = vec![
        Cw20Coin {
            address: buyer_one.to_string(),
            amount: buyer_one_original_balance,
        },
        Cw20Coin {
            address: buyer_two.to_string(),
            amount: buyer_two_original_balance,
        },
        Cw20Coin {
            address: owner.to_string(),
            amount: owner_original_balance,
        },
    ];

    let cw20_init_msg = mock_cw20_instantiate_msg(
        None,
        "Test Tokens".to_string(),
        "TTT".to_string(),
        6,
        initial_balances,
        Some(mock_minter(
            owner.to_string(),
            Some(Uint128::from(1000000u128)),
        )),
        None,
        andr.kernel.addr().to_string(),
    );
    let cw20_component = AppComponent::new(
        "cw20".to_string(),
        "cw20".to_string(),
        to_json_binary(&cw20_init_msg).unwrap(),
    );

    let auction_init_msg = mock_auction_instantiate_msg(
        None,
        andr.kernel.addr().to_string(),
        None,
        Some(vec![AndrAddr::from_string(format!(
            "./{}",
            cw721_component.name
        ))]),
        Some(AndrAddr::from_string(format!("./{}", cw20_component.name))),
    );
    let auction_component = AppComponent::new(
        "auction".to_string(),
        "auction".to_string(),
        to_json_binary(&auction_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![
        auction_component.clone(),
        cw721_component.clone(),
        cw20_component.clone(),
    ];

    let app = MockAppContract::instantiate(
        andr.get_code_id(&mut router, "app-contract"),
        owner,
        &mut router,
        "Auction App",
        app_components.clone(),
        andr.kernel.addr(),
        Some(owner.to_string()),
    );
    let components = app.query_components(&router);
    assert_eq!(components, app_components);
    let cw20: MockCW20 = app.query_ado_by_component_name(&router, cw20_component.name);

    // Mint Tokens
    let cw721: MockCW721 = app.query_ado_by_component_name(&router, cw721_component.name);

    let mint_msg = mock_quick_mint_msg(1, owner.to_string());

    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(cw721.addr().clone()),
            &mint_msg,
            &[],
        )
        .unwrap();

    // Send Token to Auction
    let auction: MockAuction = app.query_ado_by_component_name(&router, auction_component.name);

    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(auction.addr().clone()),
            &mock_authorize_token_address(cw721.addr().clone(), None),
            &[],
        )
        .unwrap();

    let start_time = router.block_info().time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO + 100;
    let receive_msg = mock_start_auction(
        Some(Milliseconds(start_time)),
        Milliseconds(start_time + 2),
        cw20.addr().to_string(),
        true,
        None,
        None,
        None,
    );

    let send_msg = mock_send_nft(
        AndrAddr::from_string("./auction".to_string()),
        "0".to_string(),
        to_json_binary(&receive_msg).unwrap(),
    );

    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(cw721.addr().clone()),
            &send_msg,
            &[],
        )
        .unwrap();

    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: Timestamp::from_nanos(start_time * MILLISECONDS_TO_NANOSECONDS_RATIO),
        chain_id: router.block_info().chain_id,
    });

    // Query Auction State
    let auction_ids_response: AuctionIdsResponse = router
        .wrap()
        .query_wasm_smart(
            auction.addr().clone(),
            &mock_get_auction_ids("0".to_string(), cw721.addr().to_string()),
        )
        .unwrap();

    assert_eq!(auction_ids_response.auction_ids.len(), 1);

    let auction_id = auction_ids_response.auction_ids.first().unwrap();
    let auction_state: AuctionStateResponse = router
        .wrap()
        .query_wasm_smart(auction.addr().clone(), &mock_get_auction_state(*auction_id))
        .unwrap();

    assert_eq!(auction_state.coin_denom, cw20.addr().to_string());

    // Place Bid One
    // Blacklist bidder now
    let actor = AndrAddr::from_string(buyer_one.clone());
    let action = "PlaceBid".to_string();
    let permission = Permission::blacklisted(None);
    let permissioning_message = mock_set_permission(actor, action, permission);

    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(auction.addr().clone()),
            &permissioning_message,
            &[],
        )
        .unwrap();

    let bid_msg = mock_place_bid("0".to_string(), cw721.addr().to_string());

    // Bid should be rejected because we blacklisted bidder one
    let err: ContractError = router
        .execute_contract(
            buyer_one.clone(),
            Addr::unchecked(auction.addr().clone()),
            &bid_msg,
            &[coin(50, "uandr")],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Now whitelist bidder one
    let actor = AndrAddr::from_string(buyer_one.clone());
    let action = "PlaceBid".to_string();
    let permission = Permission::whitelisted(None);
    let permissioning_message = mock_set_permission(actor, action, permission);

    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(auction.addr().clone()),
            &permissioning_message,
            &[],
        )
        .unwrap();

    // Try bidding again
    let hook_msg = Cw20HookMsg::PlaceBid {
        token_id: "0".to_owned(),
        token_address: cw721.addr().clone().to_string(),
    };

    let bid_msg = mock_cw20_send(
        AndrAddr::from_string(auction.addr().clone()),
        Uint128::new(50),
        to_json_binary(&hook_msg).unwrap(),
    );

    router
        .execute_contract(
            buyer_one.clone(),
            Addr::unchecked(cw20.addr().clone()),
            &bid_msg,
            &[],
        )
        .unwrap();

    // Check Bid Status One
    let bids_resp: BidsResponse = router
        .wrap()
        .query_wasm_smart(auction.addr().clone(), &mock_get_bids(*auction_id))
        .unwrap();
    assert_eq!(bids_resp.bids.len(), 1);

    let bid = bids_resp.bids.first().unwrap();
    assert_eq!(bid.bidder, buyer_one.to_string());
    assert_eq!(bid.amount, Uint128::from(50u128));

    // Second bid by buyer_two
    let bid_msg = mock_cw20_send(
        AndrAddr::from_string(auction.addr().clone()),
        Uint128::new(100),
        to_json_binary(&hook_msg).unwrap(),
    );

    router
        .execute_contract(
            buyer_two.clone(),
            Addr::unchecked(cw20.addr().clone()),
            &bid_msg,
            &[],
        )
        .unwrap();

    // Check Bid Status One
    let bids_resp: BidsResponse = router
        .wrap()
        .query_wasm_smart(auction.addr().clone(), &mock_get_bids(*auction_id))
        .unwrap();
    assert_eq!(bids_resp.bids.len(), 2);

    let bid_two = bids_resp.bids.get(1).unwrap();
    assert_eq!(bid_two.bidder, buyer_two.to_string());
    assert_eq!(bid_two.amount, Uint128::from(100u128));

    // End Auction
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: Timestamp::from_nanos((start_time + 1001) * MILLISECONDS_TO_NANOSECONDS_RATIO),
        chain_id: router.block_info().chain_id,
    });
    let end_msg = mock_claim_auction("0".to_string(), cw721.addr().to_string());

    router
        .execute_contract(
            buyer_two.clone(),
            Addr::unchecked(auction.addr()),
            &end_msg,
            &[],
        )
        .unwrap();

    // Check Final State
    let owner_resp: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(cw721.addr(), &mock_cw721_owner_of("0".to_string(), None))
        .unwrap();
    assert_eq!(owner_resp.owner, buyer_two.to_string());

    // The auction's owner sold the NFT for 100, so the balance should increase by 100
    let cw20_balance_query = mock_get_cw20_balance(owner);
    let cw20_balance_response: BalanceResponse = router
        .wrap()
        .query_wasm_smart(cw20.addr().clone(), &cw20_balance_query)
        .unwrap();
    assert_eq!(
        cw20_balance_response.balance,
        owner_original_balance
            .checked_add(Uint128::new(100))
            .unwrap()
    );

    // Buyer two won the auction with a bid of 100, the balance should be 100 less than the original balance
    let cw20_balance_query = mock_get_cw20_balance(buyer_two);
    let cw20_balance_response: BalanceResponse = router
        .wrap()
        .query_wasm_smart(cw20.addr().clone(), &cw20_balance_query)
        .unwrap();
    assert_eq!(
        cw20_balance_response.balance,
        buyer_two_original_balance
            .checked_sub(Uint128::new(100))
            .unwrap()
    );

    // Buyer one was outbid, so the balance should remain unchanged
    let cw20_balance_query = mock_get_cw20_balance(buyer_one);
    let cw20_balance_response: BalanceResponse = router
        .wrap()
        .query_wasm_smart(cw20.addr(), &cw20_balance_query)
        .unwrap();
    assert_eq!(cw20_balance_response.balance, buyer_one_original_balance);
}
