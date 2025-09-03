use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, MockAppContract};
use andromeda_auction::mock::{
    mock_andromeda_auction, mock_auction_instantiate_msg, mock_place_bid, mock_start_auction,
    MockAuction,
};
use andromeda_cw20::mock::{mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_minter, MockCW20};
use andromeda_cw721::mock::{mock_andromeda_cw721, mock_cw721_instantiate_msg, MockCW721};

use andromeda_non_fungible_tokens::auction::{AuctionStateResponse, QueryMsg};
use andromeda_rates::mock::mock_andromeda_rates;
use andromeda_splitter::mock::mock_andromeda_splitter;
use andromeda_std::{
    ado_base::permissioning::{
        LocalPermission, Permission, PermissionedActionExpirationResponse,
        PermissionedActionsResponse, PermissionedActionsWithExpirationResponse,
    },
    amp::AndrAddr,
    common::{
        denom::Asset,
        expiration::{Expiry, MILLISECONDS_TO_NANOSECONDS_RATIO},
        schedule::Schedule,
        Milliseconds,
    },
    error::ContractError,
};
use andromeda_testing::{
    mock::mock_app, mock_builder::MockAndromedaBuilder, mock_contract::MockContract,
};
use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Timestamp, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::Executor;
#[test]
fn test_permissions() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![]),
            ("buyer_one", vec![coin(1000, "uandr")]),
            ("buyer_two", vec![coin(1000, "uandr")]),
            ("buyer_three", vec![coin(1000, "uandr")]),
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
    let buyer_three = andr.get_wallet("buyer_three");
    // Generate App Components
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        AndrAddr::from_string(owner.to_string()),
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
        initial_balances.clone(),
        Some(mock_minter(
            owner.to_string(),
            Some(Uint128::from(1000000u128)),
        )),
        andr.kernel.addr().to_string(),
    );
    let cw20_component = AppComponent::new(
        "cw20".to_string(),
        "cw20".to_string(),
        to_json_binary(&cw20_init_msg).unwrap(),
    );

    let second_cw20_init_msg = mock_cw20_instantiate_msg(
        None,
        "Second Test Tokens".to_string(),
        "STTT".to_string(),
        6,
        initial_balances,
        Some(mock_minter(
            owner.to_string(),
            Some(Uint128::from(1000000u128)),
        )),
        andr.kernel.addr().to_string(),
    );
    let second_cw20_component = AppComponent::new(
        "second-cw20".to_string(),
        "cw20".to_string(),
        to_json_binary(&second_cw20_init_msg).unwrap(),
    );

    let auction_init_msg = mock_auction_instantiate_msg(
        andr.kernel.addr().to_string(),
        None,
        Some(vec![AndrAddr::from_string(format!(
            "./{}",
            cw721_component.name
        ))]),
        Some(vec![AndrAddr::from_string(format!(
            "./{}",
            cw20_component.name
        ))]),
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
        second_cw20_component.clone(),
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
    for i in 1..=2 {
        cw721
            .execute_mint(
                &mut router,
                owner.clone(),
                i.to_string(),
                AndrAddr::from_string(owner.to_string()),
            )
            .unwrap();
    }

    // Authorize NFT contract
    let auction: MockAuction = app.query_ado_by_component_name(&router, auction_component.name);
    auction
        .execute_authorize_token_address(&mut router, owner.clone(), cw721.addr(), None)
        .unwrap();

    // Send Token to Auction
    let start_time =
        Milliseconds(router.block_info().time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO + 100);
    cw721
        .execute_send_nft(
            &mut router,
            owner.clone(),
            AndrAddr::from_string("./auction".to_string()),
            "1",
            &mock_start_auction(
                Schedule::new(
                    Some(Expiry::AtTime(start_time)),
                    Some(Expiry::AtTime(
                        start_time.plus_milliseconds(Milliseconds(2)),
                    )),
                ),
                None,
                Asset::Cw20Token(AndrAddr::from_string(cw20.addr().to_string())),
                None,
                None,
                None,
                None,
                None,
            ),
        )
        .unwrap();

    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: Timestamp::from_nanos(start_time.0 * MILLISECONDS_TO_NANOSECONDS_RATIO),
        chain_id: router.block_info().chain_id,
    });

    // Query Auction State
    let auction_ids =
        auction.query_auction_ids(&mut router, "1".to_string(), cw721.addr().to_string());
    assert_eq!(auction_ids.len(), 1);

    let auction_id = auction_ids.first().unwrap();
    let auction_state: AuctionStateResponse = auction.query_auction_state(&mut router, *auction_id);
    assert_eq!(auction_state.coin_denom, cw20.addr().to_string());

    // Try to set permission with an empty vector of actors
    let actors = vec![];
    let action = "PlaceBid".to_string();
    let permission = Permission::Local(LocalPermission::blacklisted(Schedule::new(None, None)));
    let err: ContractError = auction
        .execute_set_permission(&mut router, owner.clone(), actors, action, permission)
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::NoActorsProvided {});

    // Place Bid One
    // Blacklist bidder now and blacklist bidder three just to test permissioning multiple actors at the same time
    let actors = vec![
        AndrAddr::from_string(buyer_one.clone()),
        AndrAddr::from_string(buyer_three.clone()),
    ];
    let action = "PlaceBid".to_string();
    let permission = Permission::Local(LocalPermission::blacklisted(Schedule::new(None, None)));
    auction
        .execute_set_permission(&mut router, owner.clone(), actors, action, permission)
        .unwrap();

    // Query permissioned actions
    let permissioned_actions: PermissionedActionsResponse =
        auction.query(&router, QueryMsg::PermissionedActions {});
    assert_eq!(
        PermissionedActionsResponse {
            actions: vec!["SEND_CW20".to_string(), "SEND_NFT".to_string()]
        },
        permissioned_actions
    );
    let permissioned_actions_with_expiration: PermissionedActionsWithExpirationResponse =
        auction.query(&router, QueryMsg::PermissionedActionsWithExpiration {});
    assert_eq!(
        PermissionedActionsWithExpirationResponse {
            actions_expiration: vec![
                ("SEND_CW20".to_string(), None),
                ("SEND_NFT".to_string(), None)
            ]
        },
        permissioned_actions_with_expiration
    );
    let permissioned_actions_expiration: PermissionedActionExpirationResponse = auction.query(
        &router,
        QueryMsg::PermissionedActionsExpiration {
            action: "SEND_CW20".to_string(),
        },
    );
    assert_eq!(permissioned_actions_expiration.expiration, None);

    let bid_msg = mock_place_bid("1".to_string(), cw721.addr().to_string());

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

    // Bid should be rejected because we blacklisted bidder three
    let err: ContractError = router
        .execute_contract(
            buyer_three.clone(),
            Addr::unchecked(auction.addr().clone()),
            &bid_msg,
            &[coin(50, "uandr")],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});
}
