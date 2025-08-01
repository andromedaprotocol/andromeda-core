use andromeda_address_list::mock::{
    mock_address_list_instantiate_msg, mock_andromeda_address_list, mock_query_permission_msg,
    MockAddressList,
};
use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, MockAppContract};
use andromeda_cw721::mock::{mock_andromeda_cw721, mock_cw721_instantiate_msg, MockCW721};
use andromeda_marketplace::mock::{
    mock_andromeda_marketplace, mock_buy_token, mock_marketplace_instantiate_msg,
    mock_receive_packet, mock_start_sale, MockMarketplace,
};
use andromeda_modules::address_list::{ActorPermissionResponse, PERMISSION_ACTORS_ACTION};
use andromeda_std::common::schedule::Schedule;
use andromeda_std::{
    ado_base::permissioning::{LocalPermission, Permission},
    amp::{
        messages::{AMPMsg, AMPPkt},
        AndrAddr,
    },
    common::{denom::Asset, Milliseconds},
    error::ContractError,
};
use andromeda_testing::{
    mock::mock_app, mock_builder::MockAndromedaBuilder, MockADO, MockContract,
};
use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Uint128};
use cw_multi_test::Executor;

#[test]
fn test_permission_window() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![]),
            ("buyer", vec![coin(200, "uandr")]),
            ("receiver", vec![]),
        ])
        .with_contracts(vec![
            ("app-contract", mock_andromeda_app()),
            ("cw721", mock_andromeda_cw721()),
            ("marketplace", mock_andromeda_marketplace()),
            ("address-list", mock_andromeda_address_list()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let buyer = andr.get_wallet("buyer");

    // Generate App Components
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        AndrAddr::from_string(owner.to_string()),
        andr.kernel.addr().to_string(),
        None,
    );
    let cw721_component = AppComponent::new(
        "tokens".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    // Permission the marketplace contract that will
    let address_list_init_msg =
        mock_address_list_instantiate_msg(andr.kernel.addr().to_string(), None, None);

    let address_list_component = AppComponent::new(
        "address-list",
        "address-list",
        to_json_binary(&address_list_init_msg).unwrap(),
    );

    let marketplace_init_msg =
        mock_marketplace_instantiate_msg(andr.kernel.addr().to_string(), None, None, None);
    let marketplace_component = AppComponent::new(
        "marketplace".to_string(),
        "marketplace".to_string(),
        to_json_binary(&marketplace_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![
        cw721_component.clone(),
        address_list_component.clone(),
        marketplace_component.clone(),
    ];
    let app_code_id = andr.get_code_id(&mut router, "app-contract");
    let app = MockAppContract::instantiate(
        app_code_id,
        owner,
        &mut router,
        "Marketplace App",
        app_components.clone(),
        andr.kernel.addr(),
        None,
    );

    let components = app.query_components(&router);
    assert_eq!(components, app_components);

    let cw721: MockCW721 = app.query_ado_by_component_name(&router, cw721_component.name);
    let marketplace: MockMarketplace =
        app.query_ado_by_component_name(&router, marketplace_component.name);
    let address_list: MockAddressList =
        app.query_ado_by_component_name(&router, address_list_component.name);

    // Mint Tokens
    cw721
        .execute_quick_mint(
            &mut router,
            owner.clone(),
            1,
            AndrAddr::from_string(owner.to_string()),
        )
        .unwrap();
    let token_id = "1";

    // Send Token to Marketplace
    cw721
        .execute_send_nft(
            &mut router,
            owner.clone(),
            marketplace.addr().clone(),
            token_id,
            &mock_start_sale(
                Uint128::from(100u128),
                Asset::NativeToken("uandr".to_string()),
                Schedule::new(None, None),
                None,
            ),
        )
        .unwrap();

    // Buy Token
    let buy_msg = mock_buy_token(cw721.addr().clone(), token_id);
    let amp_msg = AMPMsg::new(
        Addr::unchecked(marketplace.addr().clone()),
        to_json_binary(&buy_msg).unwrap(),
        Some(vec![coin(200, "uandr")]),
    );

    let packet = AMPPkt::new(buyer.clone(), buyer.clone(), vec![amp_msg]);
    let receive_packet_msg = mock_receive_packet(packet);

    let block_info = router.block_info();
    router.set_block(BlockInfo {
        height: block_info.height,
        time: block_info.time.plus_minutes(1),
        chain_id: block_info.chain_id,
    });

    // Blacklist buyer in address list
    address_list
        .execute_actor_permission(
            &mut router,
            owner.clone(),
            vec![AndrAddr::from_string(buyer.clone())],
            LocalPermission::blacklisted(Schedule::new(None, None)),
        )
        .unwrap();

    // Blacklist buyer using contract permission
    marketplace
        .execute_set_permissions(
            &mut router,
            owner.clone(),
            vec![AndrAddr::from_string(buyer.clone())],
            "Buy",
            Permission::Contract(AndrAddr::from_string(address_list.addr())),
        )
        .unwrap();

    // Should return Unauthorized error
    let err: ContractError = router
        .execute_contract(
            buyer.clone(),
            Addr::unchecked(marketplace.addr()),
            &mock_buy_token(cw721.addr(), token_id),
            &[coin(100, "uandr")],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Permission action for it to become strict
    marketplace
        .execute_permission_action(&mut router, owner.clone(), "Buy", None)
        .unwrap();

    let current_time = router.block_info().time;

    // Whitelist buyer with window
    address_list
        .execute_actor_permission(
            &mut router,
            owner.clone(),
            vec![AndrAddr::from_string(buyer.clone())],
            LocalPermission::whitelisted(
                Schedule::new(None, None),
                // 1 hour cooldown for each action
                Some(Milliseconds::from_seconds(3600)),
                // Last used 1 minute ago
                Some(Milliseconds::from_seconds(
                    current_time.minus_minutes(1).seconds(),
                )),
            ),
        )
        .unwrap();

    let err: ContractError = router
        .execute_contract(
            buyer.clone(),
            Addr::unchecked(marketplace.addr()),
            &receive_packet_msg,
            &[coin(100, "uandr")],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Set valid window and last used
    let valid_permission = LocalPermission::whitelisted(
        Schedule::new(None, None),
        // 1 hour cooldown for each action
        Some(Milliseconds::from_seconds(3600)),
        // Last used 2 hours ago
        Some(Milliseconds::from_seconds(
            current_time.minus_hours(2).seconds(),
        )),
    );
    address_list
        .execute_actor_permission(
            &mut router,
            owner.clone(),
            vec![AndrAddr::from_string(buyer.clone())],
            valid_permission.clone(),
        )
        .unwrap();

    // Query the permission
    let query_msg = mock_query_permission_msg(buyer.clone());

    let query: ActorPermissionResponse = address_list.query(&router, query_msg);
    let expected_permission = LocalPermission::Whitelisted {
        schedule: Schedule::new(
            Some(andromeda_std::common::expiration::Expiry::AtTime(
                Milliseconds(1571797479879),
            )),
            None,
        ),
        window: Some(Milliseconds::from_seconds(3600)),
        last_used: Some(Milliseconds::from_seconds(
            current_time.minus_hours(2).seconds(),
        )),
    };
    assert_eq!(query.permission, expected_permission);

    // Permission the marketplace contract for it to be able to call the address list
    address_list
        .execute_set_permission_actor(
            &mut router,
            owner.clone(),
            vec![AndrAddr::from_string("./marketplace")],
            PERMISSION_ACTORS_ACTION.to_string(),
            Permission::Local(LocalPermission::whitelisted(
                Schedule::new(None, None),
                None,
                None,
            )),
        )
        .unwrap();

    router
        .execute_contract(
            buyer.clone(),
            Addr::unchecked(marketplace.addr()),
            &receive_packet_msg,
            &[coin(100, "uandr")],
        )
        .unwrap();

    // Check final state
    let owner_of_token = cw721.query_owner_of(&router, token_id);
    assert_eq!(owner_of_token.to_string(), buyer.to_string());

    let balance = router.wrap().query_balance(owner, "uandr").unwrap();
    assert_eq!(balance.amount, Uint128::new(100u128));

    // Query the permission
    let query_msg = mock_query_permission_msg(buyer.clone());

    let query: ActorPermissionResponse = address_list.query(&router, query_msg);
    let expected_permission = LocalPermission::Whitelisted {
        schedule: Schedule::new(
            Some(andromeda_std::common::expiration::Expiry::AtTime(
                Milliseconds(1571797479879),
            )),
            None,
        ),
        window: Some(Milliseconds::from_seconds(3600)),
        last_used: Some(Milliseconds::from_seconds(current_time.seconds())),
    };
    assert_eq!(query.permission, expected_permission);
}
