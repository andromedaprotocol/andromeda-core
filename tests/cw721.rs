use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, mock_claim_ownership_msg, MockAppContract};
use andromeda_auction::mock::{
    mock_andromeda_auction, mock_auction_instantiate_msg, mock_start_auction, MockAuction,
};
use andromeda_cw721::mock::{mock_andromeda_cw721, mock_cw721_instantiate_msg, MockCW721};

use andromeda_non_fungible_tokens::cw721::BatchSendMsg;
use andromeda_std::{
    amp::AndrAddr,
    common::{denom::Asset, expiration::Expiry, Milliseconds},
};
use andromeda_testing::{
    mock::mock_app, mock_builder::MockAndromedaBuilder, mock_contract::MockContract,
};
use cosmwasm_std::{to_json_binary, Addr, Uint128};
use cw_multi_test::Executor;

#[test]
fn test_cw721_batch_send() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![("owner", vec![])])
        .with_contracts(vec![
            ("cw721", mock_andromeda_cw721()),
            ("auction", mock_andromeda_auction()),
            ("app-contract", mock_andromeda_app()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
            
    // Generate App Components
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        owner.to_string(),
        andr.kernel.addr().to_string(),
        None,
    );
    let cw721_component = AppComponent::new(
        "cw721".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    let auction_init_msg =
        mock_auction_instantiate_msg(andr.kernel.addr().to_string(), None, None, None);
    let auction_component = AppComponent::new(
        "auction".to_string(),
        "auction".to_string(),
        to_json_binary(&auction_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![cw721_component.clone(), auction_component.clone()];
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

    // Mint 2 NFTs
    let cw721: MockCW721 = app.query_ado_by_component_name(&router, cw721_component.name);
    cw721
        .execute_quick_mint(&mut router, owner.clone(), 2, owner.to_string())
        .unwrap();

    // Send Token to Auction
    let auction: MockAuction = app.query_ado_by_component_name(&router, auction_component.name);

    let start_time = Milliseconds::from_nanos(router.block_info().time.nanos())
        .plus_milliseconds(Milliseconds(100));
    let receive_msg_1 = mock_start_auction(
        Some(Expiry::AtTime(start_time)),
        Expiry::AtTime(start_time.plus_milliseconds(Milliseconds(1000))),
        None,
        Asset::NativeToken("uandr".to_string()),
        None,
        None,
        None,
        None,
    );
    let receive_msg_2 = mock_start_auction(
        Some(Expiry::AtTime(start_time)),
        Expiry::AtTime(start_time.plus_milliseconds(Milliseconds(1000))),
        None,
        Asset::NativeToken("uandr".to_string()),
        Some(Uint128::one()),
        None,
        None,
        None,
    );

    let batch = vec![
        BatchSendMsg {
            token_id: "1".to_string(),
            contract_addr: AndrAddr::from_string(auction.addr().to_string()),
            msg: to_json_binary(&receive_msg_1).unwrap(),
        },
        BatchSendMsg {
            token_id: "2".to_string(),
            contract_addr: AndrAddr::from_string(auction.addr().to_string()),
            msg: to_json_binary(&receive_msg_2).unwrap(),
        },
    ];
    cw721
        .execute_batch_send_nft(&mut router, owner.clone(), batch)
        .unwrap();

    // Query Auction State
    let mut auction_ids: Vec<Uint128> =
        auction.query_auction_ids(&mut router, "1".to_string(), cw721.addr().to_string());

    // Append auction id of token 2
    let auction_ids_2: Vec<Uint128> =
        auction.query_auction_ids(&mut router, "2".to_string(), cw721.addr().to_string());

    auction_ids.extend(auction_ids_2);

    assert_eq!(auction_ids.len(), 2);

    let auction_id = auction_ids.first().unwrap();
    let auction_state = auction.query_auction_state(&mut router, *auction_id);

    assert_eq!(auction_state.coin_denom, "uandr".to_string());
    assert_eq!(auction_state.owner, owner.to_string());
    assert_eq!(auction_state.min_bid, None);

    let auction_id_2 = auction_ids.last().unwrap();
    let auction_state_2 = auction.query_auction_state(&mut router, *auction_id_2);

    assert_eq!(auction_state_2.coin_denom, "uandr".to_string());
    // The distinguishing factor between the two auctions is the min_bid
    assert_eq!(auction_state_2.min_bid, Some(Uint128::one()));
}
