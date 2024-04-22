#![cfg(not(target_arch = "wasm32"))]

use andromeda_address_list::mock::{
    mock_add_address_msg, mock_address_list_instantiate_msg, mock_andromeda_address_list,
    MockAddressList,
};
use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, MockAppContract};
use andromeda_cw20::mock::{
    mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_cw20_send, mock_get_cw20_balance,
    mock_minter, MockCW20,
};
use andromeda_cw721::mock::{
    mock_andromeda_cw721, mock_cw721_instantiate_msg, mock_cw721_owner_of, mock_quick_mint_msg,
    mock_send_nft, MockCW721,
};
use andromeda_finance::splitter::AddressPercent;
use andromeda_marketplace::mock::{
    mock_andromeda_marketplace, mock_buy_token, mock_marketplace_instantiate_msg,
    mock_receive_packet, mock_start_sale, MockMarketplace,
};
use andromeda_modules::rates::{Rate, RateInfo};

use andromeda_non_fungible_tokens::marketplace::Cw20HookMsg;
use andromeda_rates::mock::{mock_andromeda_rates, mock_rates_instantiate_msg};
use andromeda_splitter::mock::{
    mock_andromeda_splitter, mock_splitter_instantiate_msg, mock_splitter_send_msg,
};
use andromeda_std::ado_base::modules::Module;
use andromeda_std::amp::messages::{AMPMsg, AMPPkt};
use andromeda_std::amp::{AndrAddr, Recipient};
use andromeda_testing::mock::mock_app;
use andromeda_testing::mock_builder::MockAndromedaBuilder;
use andromeda_testing::MockContract;
use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Decimal, Uint128};
use cw20::{BalanceResponse, Cw20Coin};
use cw721::OwnerOfResponse;
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
        None,
        andr.kernel.addr().to_string(),
        None,
    );
    let cw721_component = AppComponent::new(
        "tokens".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    let rates: Vec<RateInfo> = vec![RateInfo {
        rate: Rate::Flat(coin(100, "uandr")),
        is_additive: true,
        description: None,
        recipients: vec![Recipient::from_string(rates_receiver.to_string())],
    }];
    let rates_init_msg = mock_rates_instantiate_msg(rates, andr.kernel.addr().to_string(), None);
    let rates_component =
        AppComponent::new("rates", "rates", to_json_binary(&rates_init_msg).unwrap());

    let address_list_init_msg =
        mock_address_list_instantiate_msg(true, andr.kernel.addr().to_string(), None);
    mock_address_list_instantiate_msg(true, andr.kernel.addr().to_string(), None);
    let address_list_component = AppComponent::new(
        "address-list",
        "address-list",
        to_json_binary(&address_list_init_msg).unwrap(),
    );

    let modules: Vec<Module> = vec![
        Module::new("rates", format!("./{}", rates_component.name), false),
        Module::new(
            "address-list",
            format!("./{}", address_list_component.name),
            false,
        ),
    ];
    let marketplace_init_msg =
        mock_marketplace_instantiate_msg(andr.kernel.addr().to_string(), Some(modules), None, None);
    let marketplace_component = AppComponent::new(
        "marketplace".to_string(),
        "marketplace".to_string(),
        to_json_binary(&marketplace_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![
        cw721_component.clone(),
        rates_component,
        address_list_component.clone(),
        marketplace_component.clone(),
    ];
    let app_code_id = andr.get_code_id(&mut router, "app-contract");
    let app = MockAppContract::instantiate(
        app_code_id,
        owner,
        &mut router,
        "Auction App",
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
        .execute_quick_mint(&mut router, owner.clone(), 1, owner.to_string())
        .unwrap();
    let token_id = "0";

    // Whitelist
    address_list
        .execute_add_address(&mut router, owner.clone(), cw721.addr())
        .unwrap();
    address_list
        .execute_add_address(&mut router, owner.clone(), buyer.to_string())
        .unwrap();

    // Send Token to Marketplace
    cw721
        .execute_send_nft(
            &mut router,
            owner.clone(),
            marketplace.addr().clone(),
            token_id,
            &mock_start_sale(Uint128::from(100u128), "uandr", false, None, None, None),
        )
        .unwrap();

    // Buy Token
    let buy_msg = mock_buy_token(cw721.addr().clone(), token_id);
    let amp_msg = AMPMsg::new(
        Addr::unchecked(marketplace.addr().clone()),
        to_json_binary(&buy_msg).unwrap(),
        Some(vec![coin(200, "uandr")]),
    );

    let packet = AMPPkt::new(buyer.clone(), andr.kernel.addr().to_string(), vec![amp_msg]);
    let receive_packet_msg = mock_receive_packet(packet);

    let block_info = router.block_info();
    router.set_block(BlockInfo {
        height: block_info.height,
        time: block_info.time.plus_minutes(1),
        chain_id: block_info.chain_id,
    });

    router
        .execute_contract(
            buyer.clone(),
            Addr::unchecked(marketplace.addr()),
            &receive_packet_msg,
            &[coin(200, "uandr")],
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
}

#[test]
fn test_marketplace_app_recipient() {
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
            ("splitter", mock_andromeda_splitter()),
            ("address-list", mock_andromeda_address_list()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let buyer = andr.get_wallet("buyer");
    let receiver = andr.get_wallet("receiver");

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
        "tokens".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    let splitter_init_msg = mock_splitter_instantiate_msg(
        vec![AddressPercent::new(
            Recipient::from_string(receiver),
            Decimal::one(),
        )],
        andr.kernel.addr(),
        None,
        None,
    );
    let splitter_component = AppComponent::new(
        "splitter",
        "splitter",
        to_json_binary(&splitter_init_msg).unwrap(),
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
        splitter_component.clone(),
        marketplace_component.clone(),
    ];
    let app_code_id = andr.get_code_id(&mut router, "app-contract");
    let app = MockAppContract::instantiate(
        app_code_id,
        owner,
        &mut router,
        "Auction App",
        app_components.clone(),
        andr.kernel.addr(),
        None,
    );

    let components = app.query_components(&router);
    assert_eq!(components, app_components);

    let cw721: MockCW721 = app.query_ado_by_component_name(&router, cw721_component.name);
    let marketplace: MockMarketplace =
        app.query_ado_by_component_name(&router, marketplace_component.name);

    // Mint Tokens
    cw721
        .execute_quick_mint(&mut router, owner.clone(), 1, owner.to_string())
        .unwrap();
    let token_id = "0";

    // Send Token to Marketplace
    cw721
        .execute_send_nft(
            &mut router,
            owner.clone(),
            marketplace.addr().clone(),
            token_id,
            &mock_start_sale(
                Uint128::from(100u128),
                "uandr",
                false,
                None,
                None,
                Some(
                    Recipient::from_string(format!("./{}", splitter_component.name))
                        .with_msg(mock_splitter_send_msg()),
                ),
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

    let packet = AMPPkt::new(buyer.clone(), andr.kernel.addr().to_string(), vec![amp_msg]);
    let receive_packet_msg = mock_receive_packet(packet);

    let block_info = router.block_info();
    router.set_block(BlockInfo {
        height: block_info.height,
        time: block_info.time.plus_minutes(1),
        chain_id: block_info.chain_id,
    });

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
    assert_eq!(owner_of_token, buyer.to_string());

    let balance = router.wrap().query_balance(receiver, "uandr").unwrap();
    assert_eq!(balance.amount, Uint128::from(100u128));
}
#[test]
fn test_marketplace_app_cw20() {
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
            ("cw20", mock_andromeda_cw20()),
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
        None,
        andr.kernel.addr().to_string(),
        None,
    );
    let cw721_component = AppComponent::new(
        "tokens".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    let owner_original_balance = Uint128::new(1_000);
    let buyer_original_balance = Uint128::new(2_000);
    let initial_balances = vec![
        Cw20Coin {
            address: owner.to_string(),
            amount: owner_original_balance,
        },
        Cw20Coin {
            address: buyer.to_string(),
            amount: buyer_original_balance,
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

    let rates: Vec<RateInfo> = vec![RateInfo {
        rate: Rate::Flat(coin(100, "uandr")),
        is_additive: true,
        description: None,
        recipients: vec![Recipient::from_string(rates_receiver.to_string())],
    }];
    let rates_init_msg = mock_rates_instantiate_msg(rates, andr.kernel.addr().to_string(), None);
    let rates_component =
        AppComponent::new("rates", "rates", to_json_binary(&rates_init_msg).unwrap());

    let address_list_init_msg =
        mock_address_list_instantiate_msg(true, andr.kernel.addr().to_string(), None);
    let address_list_component = AppComponent::new(
        "address-list",
        "address-list",
        to_json_binary(&address_list_init_msg).unwrap(),
    );

    let modules: Vec<Module> = vec![
        Module::new("rates", format!("./{}", rates_component.name), false),
        Module::new(
            "address-list",
            format!("./{}", address_list_component.name),
            false,
        ),
    ];
    let marketplace_init_msg = mock_marketplace_instantiate_msg(
        andr.kernel.addr().to_string(),
        Some(modules),
        None,
        Some(AndrAddr::from_string(format!("./{}", cw20_component.name))),
    );
    let marketplace_component = AppComponent::new(
        "marketplace".to_string(),
        "marketplace".to_string(),
        to_json_binary(&marketplace_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![
        cw721_component.clone(),
        cw20_component.clone(),
        rates_component,
        address_list_component.clone(),
        marketplace_component.clone(),
    ];

    let app_code_id = andr.get_code_id(&mut router, "app-contract");
    let app = MockAppContract::instantiate(
        app_code_id,
        owner,
        &mut router,
        "Auction App",
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

    let cw20: MockCW20 = app.query_ado_by_component_name(&router, cw20_component.name);

    // Mint Tokens
    let mint_msg = mock_quick_mint_msg(1, owner.to_string());
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(cw721.addr().clone()),
            &mint_msg,
            &[],
        )
        .unwrap();
    let token_id = "0";

    // Whitelist
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(address_list.addr().clone()),
            &mock_add_address_msg(cw721.addr().to_string()),
            &[],
        )
        .unwrap();
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(address_list.addr().clone()),
            &mock_add_address_msg(buyer.to_string()),
            &[],
        )
        .unwrap();
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(address_list.addr()),
            &mock_add_address_msg(cw20.addr().to_string()),
            &[],
        )
        .unwrap();

    // Send Token to Marketplace
    let send_nft_msg = mock_send_nft(
        AndrAddr::from_string(marketplace.addr().clone()),
        token_id.to_string(),
        to_json_binary(&mock_start_sale(
            Uint128::from(100u128),
            cw20.addr().clone(),
            true,
            None,
            None,
            None,
        ))
        .unwrap(),
    );
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(cw721.addr().clone()),
            &send_nft_msg,
            &[],
        )
        .unwrap();

    // Buy Token
    let hook_msg = Cw20HookMsg::Buy {
        token_id: token_id.to_owned(),
        token_address: cw721.addr().to_string(),
    };

    let buy_msg = mock_cw20_send(
        AndrAddr::from_string(marketplace.addr()),
        Uint128::new(200),
        to_json_binary(&hook_msg).unwrap(),
    );

    let block_info = router.block_info();
    router.set_block(BlockInfo {
        height: block_info.height,
        time: block_info.time.plus_minutes(1),
        chain_id: block_info.chain_id,
    });

    router
        .execute_contract(
            buyer.clone(),
            Addr::unchecked(cw20.addr().clone()),
            &buy_msg,
            &[],
        )
        .unwrap();

    // let amp_msg = AMPMsg::new(
    //     Addr::unchecked(marketplace_addr.clone()),
    //     to_json_binary(&buy_msg).unwrap(),
    //     None,
    // );

    // let packet = AMPPkt::new(
    //     buyer.clone(),
    //     andr.kernel_address.to_string(),
    //     vec![amp_msg],
    // );
    // let receive_packet_msg = mock_receive_packet(packet);

    // router
    //     .execute_contract(
    //         buyer.clone(),
    //         Addr::unchecked(marketplace_addr),
    //         &receive_packet_msg,
    //         &[coin(200, "uandr")],
    //     )
    //     .unwrap();

    // Check final state
    let owner_resp: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(
            cw721.addr(),
            &mock_cw721_owner_of(token_id.to_string(), None),
        )
        .unwrap();
    assert_eq!(owner_resp.owner, buyer.to_string());

    // The NFT owner sold it for 200, there's also a 50% tax so the owner should receive 100
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

    // Buyer bought the NFT for 200, should be 200 less
    let cw20_balance_query = mock_get_cw20_balance(buyer);
    let cw20_balance_response: BalanceResponse = router
        .wrap()
        .query_wasm_smart(cw20.addr().clone(), &cw20_balance_query)
        .unwrap();
    assert_eq!(
        cw20_balance_response.balance,
        buyer_original_balance
            .checked_sub(Uint128::new(200))
            .unwrap()
    );

    // The rates receiver should get 100 coins
    let cw20_balance_query = mock_get_cw20_balance(rates_receiver);
    let cw20_balance_response: BalanceResponse = router
        .wrap()
        .query_wasm_smart(cw20.addr(), &cw20_balance_query)
        .unwrap();
    assert_eq!(cw20_balance_response.balance, Uint128::new(100));
}
