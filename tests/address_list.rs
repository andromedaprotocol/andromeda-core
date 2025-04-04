use andromeda_address_list::mock::{
    mock_address_list_instantiate_msg, mock_andromeda_address_list, mock_query_permission_msg,
    MockAddressList,
};
use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, MockAppContract};
use andromeda_cw20::mock::{mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_minter, MockCW20};
use andromeda_cw721::mock::{mock_andromeda_cw721, mock_cw721_instantiate_msg, MockCW721};
use andromeda_finance::splitter::AddressPercent;
use andromeda_marketplace::mock::{
    mock_andromeda_marketplace, mock_buy_token, mock_marketplace_instantiate_msg,
    mock_receive_packet, mock_start_sale, MockMarketplace,
};
use andromeda_modules::address_list::ActorPermissionResponse;
use andromeda_non_fungible_tokens::marketplace::Cw20HookMsg;
use andromeda_rates::mock::{mock_andromeda_rates, mock_rates_instantiate_msg, MockRates};
use andromeda_splitter::mock::{
    mock_andromeda_splitter, mock_splitter_instantiate_msg, mock_splitter_send_msg,
};
use andromeda_std::ado_base::permissioning::{LocalPermission, Permission};
use andromeda_std::ado_base::rates::{AllRatesResponse, LocalRate};
use andromeda_std::ado_base::rates::{LocalRateType, LocalRateValue, PercentRate, Rate};
use andromeda_std::amp::messages::{AMPMsg, AMPPkt};
use andromeda_std::amp::{AndrAddr, Recipient};
use andromeda_std::common::denom::Asset;
use andromeda_std::common::Milliseconds;
use andromeda_std::error::ContractError;
use andromeda_testing::mock::mock_app;
use andromeda_testing::mock_builder::MockAndromedaBuilder;
use andromeda_testing::MockADO;
use andromeda_testing::MockContract;
use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Decimal, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::Executor;

#[test]
fn test_marketplace_app() {
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
            ("rates", mock_andromeda_rates()),
            ("address-list", mock_andromeda_address_list()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let buyer = andr.get_wallet("buyer");
    let rates_receiver = andr.get_wallet("receiver");

    // Generate App Components
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        owner.to_string(),
        andr.kernel.addr().to_string(),
        None,
    );
    let cw721_component = AppComponent::new(
        "tokens".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );
    // Set a royalty which is worth as much as the marketplace sale price
    // The sale recipient will not receive any funds because they're all going to the royalty recipient
    let local_rate = LocalRate {
        rate_type: LocalRateType::Deductive,
        recipient: Recipient::from_string(rates_receiver.to_string()),
        value: LocalRateValue::Flat(coin(100, "uandr")),
        description: None,
    };

    let rates_init_msg = mock_rates_instantiate_msg(
        "Buy".to_string(),
        local_rate,
        andr.kernel.addr().to_string(),
        None,
    );
    let rates_component =
        AppComponent::new("rates", "rates", to_json_binary(&rates_init_msg).unwrap());

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
        rates_component.clone(),
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
    let rates: MockRates = app.query_ado_by_component_name(&router, rates_component.name);

    // Set contract rate linked to the above rates contract
    marketplace
        .execute_set_rate(
            &mut router,
            owner.clone(),
            "Buy",
            Rate::Contract(AndrAddr::from_string(rates.addr())),
        )
        .unwrap();

    let rate = marketplace
        .query_rates(&mut router, "Buy".to_string())
        .unwrap();

    assert_eq!(rate, Rate::Contract(AndrAddr::from_string(rates.addr())));

    let all_rates: AllRatesResponse = marketplace.query_all_rates(&mut router);

    assert_eq!(
        all_rates,
        AllRatesResponse {
            all_rates: vec![(
                "Buy".to_string(),
                Rate::Contract(AndrAddr::from_string(rates.addr()))
            )]
        }
    );

    // Mint Tokens
    cw721
        .execute_quick_mint(&mut router, owner.clone(), 1, owner.to_string())
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
                None,
                None,
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

    // Try adding limited permission in address list, should error
    let err: ContractError = address_list
        .execute_actor_permission(
            &mut router,
            owner.clone(),
            vec![AndrAddr::from_string(buyer.clone())],
            LocalPermission::limited(None, None, 1),
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(
        err,
        ContractError::InvalidPermission {
            msg: "Limited permission is not supported in address list contract".to_string(),
        }
    );

    // Blacklist buyer in address list
    address_list
        .execute_actor_permission(
            &mut router,
            owner.clone(),
            vec![AndrAddr::from_string(buyer.clone())],
            LocalPermission::blacklisted(None, None),
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
            // We're sending the exact amount required, which is the price + tax
            &[coin(100, "uandr")],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    let current_time = router.block_info().time.seconds();
    println!("Current time: {}", current_time);

    // WHitelist buyer with frequency
    address_list
        .execute_actor_permission(
            &mut router,
            owner.clone(),
            vec![AndrAddr::from_string(buyer.clone())],
            LocalPermission::whitelisted(
                None,
                None,
                Some(Milliseconds::from_seconds(3600)),
                Some(Milliseconds::from_seconds(current_time)),
            ),
        )
        .unwrap();

    router
        .execute_contract(
            buyer.clone(),
            Addr::unchecked(marketplace.addr()),
            &receive_packet_msg,
            // We're sending the exact amount required, which is the price + tax
            &[coin(100, "uandr")],
        )
        .unwrap();

    // Check final state
    let owner_of_token = cw721.query_owner_of(&router, token_id);
    assert_eq!(owner_of_token, buyer.to_string());

    let balance = router
        .wrap()
        .query_balance(rates_receiver, "uandr")
        .unwrap();
    assert_eq!(balance.amount, Uint128::from(100u128));

    let balance = router.wrap().query_balance(owner, "uandr").unwrap();
    assert_eq!(balance.amount, Uint128::zero());
}
