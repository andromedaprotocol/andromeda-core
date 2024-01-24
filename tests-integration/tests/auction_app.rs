#![cfg(not(target_arch = "wasm32"))]

use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, MockApp};
use andromeda_auction::mock::{
    mock_andromeda_auction, mock_auction_instantiate_msg, mock_start_auction, MockAuction,
};
use andromeda_cw721::mock::{mock_andromeda_cw721, mock_cw721_instantiate_msg, MockCW721};

use andromeda_std::common::expiration::MILLISECONDS_TO_NANOSECONDS_RATIO;
use andromeda_testing::{mock::MockAndromeda, mock_contract::MockContract};
use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Timestamp, Uint128};

use cw_multi_test::App;

fn mock_app() -> App {
    App::new(|router, _api, storage| {
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked("owner"),
                [coin(100, "uandr")].to_vec(),
            )
            .unwrap();
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked("buyer_one"),
                [coin(100, "uandr")].to_vec(),
            )
            .unwrap();
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked("buyer_two"),
                [coin(100, "uandr")].to_vec(),
            )
            .unwrap();
    })
}

fn mock_andromeda(app: &mut App, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

#[test]
fn test_auction_app() {
    let owner = Addr::unchecked("owner");
    let buyer_one = Addr::unchecked("buyer_one");
    let buyer_two = Addr::unchecked("buyer_two");

    let mut router = mock_app();
    let andr = mock_andromeda(&mut router, owner.clone());
    // Store contract codes
    andr.store_ado(&mut router, mock_andromeda_cw721(), "cw721");
    andr.store_ado(&mut router, mock_andromeda_auction(), "auction");
    andr.store_ado(&mut router, mock_andromeda_app(), "app");

    // Generate App Components
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        owner.to_string(),
        None,
        andr.kernel_address.to_string(),
        None,
    );
    let cw721_component = AppComponent::new(
        "1".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    let auction_init_msg =
        mock_auction_instantiate_msg(None, andr.kernel_address.to_string(), None);
    let auction_component = AppComponent::new(
        "2".to_string(),
        "auction".to_string(),
        to_json_binary(&auction_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![cw721_component.clone(), auction_component.clone()];
    let app = MockApp::instantiate(
        andr.get_code_id(&mut router, "app"),
        owner.clone(),
        &mut router,
        "Auction App",
        app_components,
        andr.kernel_address,
        Some(owner.to_string()),
    );

    // Mint Tokens
    let cw721: MockCW721 = app.query_ado_by_component_name(&mut router, cw721_component.name);
    cw721
        .execute_quick_mint(&mut router, owner.clone(), 1, owner.to_string())
        .unwrap();

    // Send Token to Auction
    let auction: MockAuction = app.query_ado_by_component_name(&mut router, auction_component.name);
    let start_time = router.block_info().time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO + 100;
    let receive_msg = mock_start_auction(start_time, 1000, "uandr".to_string(), None, None);
    cw721
        .execute_send_nft(
            &mut router,
            owner.clone(),
            auction.addr(),
            "0",
            to_json_binary(&receive_msg).unwrap(),
        )
        .unwrap();

    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: Timestamp::from_nanos(start_time * MILLISECONDS_TO_NANOSECONDS_RATIO),
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
        time: Timestamp::from_nanos((start_time + 1001) * MILLISECONDS_TO_NANOSECONDS_RATIO),
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
    assert_eq!(owner_balance.amount, Uint128::from(200u128));
}
