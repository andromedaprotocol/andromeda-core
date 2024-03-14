#![cfg(not(target_arch = "wasm32"))]

use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{
    mock_andromeda_app, mock_app_instantiate_msg, mock_claim_ownership_msg, mock_get_address_msg,
    mock_get_components_msg,
};
use andromeda_auction::mock::{
    mock_andromeda_auction, mock_auction_instantiate_msg, mock_authorize_token_address,
    mock_claim_auction, mock_get_auction_ids, mock_get_auction_state, mock_get_bids,
    mock_place_bid, mock_receive_packet, mock_start_auction,
};
use andromeda_cw721::mock::{
    mock_andromeda_cw721, mock_cw721_instantiate_msg, mock_cw721_owner_of, mock_quick_mint_msg,
    mock_send_nft,
};
use andromeda_non_fungible_tokens::auction::{
    AuctionIdsResponse, AuctionStateResponse, BidsResponse,
};
use andromeda_std::amp::messages::{AMPMsg, AMPPkt};
use andromeda_std::common::expiration::MILLISECONDS_TO_NANOSECONDS_RATIO;
use andromeda_testing::mock::MockAndromeda;
use cosmwasm_std::{coin, to_binary, Addr, BlockInfo, Timestamp, Uint128};
use cw721::OwnerOfResponse;
use cw_multi_test::{App, Executor};

fn mock_app() -> App {
    App::new(|router, _api, storage| {
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked("owner"),
                [coin(999999, "uandr")].to_vec(),
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
    let cw721_code_id = router.store_code(mock_andromeda_cw721());
    let auction_code_id = router.store_code(mock_andromeda_auction());
    let app_code_id = router.store_code(mock_andromeda_app());
    andr.store_code_id(&mut router, "cw721", cw721_code_id);
    andr.store_code_id(&mut router, "auction", auction_code_id);
    andr.store_code_id(&mut router, "app-contract", app_code_id);

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
        "cw721".to_string(),
        "cw721".to_string(),
        to_binary(&cw721_init_msg).unwrap(),
    );

    let auction_init_msg =
        mock_auction_instantiate_msg(None, andr.kernel_address.to_string(), None, None);
    let auction_component = AppComponent::new(
        "auction".to_string(),
        "auction".to_string(),
        to_binary(&auction_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![cw721_component.clone(), auction_component.clone()];
    let app_init_msg = mock_app_instantiate_msg(
        "AuctionApp".to_string(),
        app_components.clone(),
        andr.kernel_address.clone(),
        None,
    );

    let app_addr = router
        .instantiate_contract(
            app_code_id,
            owner.clone(),
            &app_init_msg,
            &[],
            "Auction App",
            Some(owner.to_string()),
        )
        .unwrap();

    let components: Vec<AppComponent> = router
        .wrap()
        .query_wasm_smart(app_addr.clone(), &mock_get_components_msg())
        .unwrap();

    assert_eq!(components, app_components);

    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(app_addr.clone()),
            &mock_claim_ownership_msg(None),
            &[],
        )
        .unwrap();

    // Mint Tokens
    let cw721_addr: String = router
        .wrap()
        .query_wasm_smart(
            app_addr.clone(),
            &mock_get_address_msg(cw721_component.name),
        )
        .unwrap();
    let mint_msg = mock_quick_mint_msg(1, owner.to_string());
    andr.accept_ownership(&mut router, cw721_addr.clone(), owner.clone());
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(cw721_addr.clone()),
            &mint_msg,
            &[],
        )
        .unwrap();

    // Send Token to Auction
    let auction_addr: String = router
        .wrap()
        .query_wasm_smart(
            app_addr,
            &mock_get_address_msg(auction_component.name.to_string()),
        )
        .unwrap();
    andr.accept_ownership(&mut router, auction_addr.clone(), owner.clone());
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(auction_addr.clone()),
            &mock_authorize_token_address(cw721_addr.clone(), None),
            &[],
        )
        .unwrap();

    let start_time = router.block_info().time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO + 100;
    let receive_msg = mock_start_auction(start_time, 1000, "uandr".to_string(), None, None);
    let send_msg = mock_send_nft(
        auction_addr.clone(),
        "0".to_string(),
        to_binary(&receive_msg).unwrap(),
    );

    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(cw721_addr.clone()),
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
            auction_addr.clone(),
            &mock_get_auction_ids("0".to_string(), cw721_addr.clone()),
        )
        .unwrap();

    assert_eq!(auction_ids_response.auction_ids.len(), 1);

    let auction_id = auction_ids_response.auction_ids.first().unwrap();
    let auction_state: AuctionStateResponse = router
        .wrap()
        .query_wasm_smart(auction_addr.clone(), &mock_get_auction_state(*auction_id))
        .unwrap();

    assert_eq!(auction_state.coin_denom, "uandr".to_string());

    // Place Bid One
    let bid_msg = mock_place_bid("0".to_string(), cw721_addr.clone());
    let amp_msg = AMPMsg::new(
        auction_addr.clone(),
        to_binary(&bid_msg).unwrap(),
        Some(vec![coin(50, "uandr")]),
    );

    let packet = AMPPkt::new(
        buyer_one.clone(),
        andr.kernel_address.to_string(),
        vec![amp_msg],
    );
    let receive_packet_msg = mock_receive_packet(packet);

    router
        .execute_contract(
            buyer_one.clone(),
            Addr::unchecked(auction_addr.clone()),
            &receive_packet_msg,
            &[coin(50, "uandr")],
        )
        .unwrap();

    // Check Bid Status One
    let bids_resp: BidsResponse = router
        .wrap()
        .query_wasm_smart(auction_addr.clone(), &mock_get_bids(*auction_id))
        .unwrap();
    assert_eq!(bids_resp.bids.len(), 1);

    let bid = bids_resp.bids.first().unwrap();
    assert_eq!(bid.bidder, buyer_one.to_string());
    assert_eq!(bid.amount, Uint128::from(50u128));

    router
        .execute_contract(
            buyer_two.clone(),
            Addr::unchecked(auction_addr.clone()),
            &bid_msg,
            &[coin(100, "uandr")],
        )
        .unwrap();

    // Check Bid Status One
    let bids_resp: BidsResponse = router
        .wrap()
        .query_wasm_smart(auction_addr.clone(), &mock_get_bids(*auction_id))
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
    let end_msg = mock_claim_auction("0".to_string(), cw721_addr.clone());
    let seller_pre_balance = router
        .wrap()
        .query_balance(owner.clone(), "uandr".to_string())
        .unwrap();
    router
        .execute_contract(
            buyer_two.clone(),
            Addr::unchecked(auction_addr),
            &end_msg,
            &[],
        )
        .unwrap();

    // Check Final State
    let owner_resp: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(cw721_addr, &mock_cw721_owner_of("0".to_string(), None))
        .unwrap();
    assert_eq!(owner_resp.owner, buyer_two);

    let seller_post_balance = router
        .wrap()
        .query_balance(owner, "uandr".to_string())
        .unwrap();
    assert_eq!(
        seller_pre_balance.amount,
        seller_post_balance
            .amount
            .checked_sub(Uint128::from(100u128))
            .unwrap()
    );
}
