#![cfg(not(target_arch = "wasm32"))]

use andromeda_address_list::mock::{
    mock_add_address_msg, mock_address_list_instantiate_msg, mock_andromeda_address_list,
};
use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{
    mock_andromeda_app, mock_app_instantiate_msg, mock_get_address_msg, mock_get_components_msg,
};
use andromeda_cw20::mock::{
    mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_cw20_send, mock_get_cw20_balance,
    mock_minter,
};
use andromeda_cw721::mock::{
    mock_andromeda_cw721, mock_cw721_instantiate_msg, mock_cw721_owner_of, mock_quick_mint_msg,
    mock_send_nft,
};
use andromeda_marketplace::mock::{
    mock_andromeda_marketplace, mock_buy_token, mock_marketplace_instantiate_msg,
    mock_receive_packet, mock_start_sale,
};
use andromeda_modules::rates::{Rate, RateInfo};

use andromeda_non_fungible_tokens::marketplace::Cw20HookMsg;
use andromeda_rates::mock::{mock_andromeda_rates, mock_rates_instantiate_msg};
use andromeda_std::ado_base::modules::Module;
use andromeda_std::amp::messages::{AMPMsg, AMPPkt};
use andromeda_std::amp::{AndrAddr, Recipient};
use andromeda_testing::mock::{mock_app, MockAndromeda, MockApp};
use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Uint128};
use cw20::{BalanceResponse, Cw20Coin};
use cw721::OwnerOfResponse;
use cw_multi_test::Executor;

fn mock_andromeda(app: &mut MockApp, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

#[test]
fn test_marketplace_app() {
    let mut router = mock_app();
    let owner = router.api().addr_make("owner");
    let buyer = router.api().addr_make("buyer");
    let rates_receiver = router.api().addr_make("receiver");
    router
        .send_tokens(
            Addr::unchecked("owner"),
            buyer.clone(),
            &[coin(200, "uandr")],
        )
        .unwrap();

    let andr = mock_andromeda(&mut router, owner.clone());

    // Store contract codes
    let cw721_code_id = router.store_code(mock_andromeda_cw721());
    let marketplace_code_id = router.store_code(mock_andromeda_marketplace());
    let app_code_id = router.store_code(mock_andromeda_app());
    let rates_code_id = router.store_code(mock_andromeda_rates());
    let address_list_code_id = router.store_code(mock_andromeda_address_list());

    andr.store_code_id(&mut router, "cw721", cw721_code_id);
    andr.store_code_id(&mut router, "marketplace", marketplace_code_id);
    andr.store_code_id(&mut router, "rates", rates_code_id);
    andr.store_code_id(&mut router, "address-list", address_list_code_id);
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
    let rates_init_msg = mock_rates_instantiate_msg(rates, andr.kernel_address.to_string(), None);
    let rates_component =
        AppComponent::new("rates", "rates", to_json_binary(&rates_init_msg).unwrap());

    let address_list_init_msg =
        mock_address_list_instantiate_msg(true, andr.kernel_address.to_string(), None);
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
        andr.kernel_address.to_string(),
        Some(modules),
        None,
        None,
    );
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
    let app_init_msg = mock_app_instantiate_msg(
        "Auction App".to_string(),
        app_components.clone(),
        andr.kernel_address.to_string(),
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

    let cw721_addr: String = router
        .wrap()
        .query_wasm_smart(
            app_addr.clone(),
            &mock_get_address_msg(cw721_component.name),
        )
        .unwrap();
    let marketplace_addr: String = router
        .wrap()
        .query_wasm_smart(
            app_addr.clone(),
            &mock_get_address_msg(marketplace_component.name),
        )
        .unwrap();
    let address_list_addr: String = router
        .wrap()
        .query_wasm_smart(app_addr, &mock_get_address_msg(address_list_component.name))
        .unwrap();

    // Mint Tokens
    let mint_msg = mock_quick_mint_msg(1, owner.to_string());
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(cw721_addr.clone()),
            &mint_msg,
            &[],
        )
        .unwrap();

    let token_id = "0";

    // Whitelist
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(address_list_addr.clone()),
            &mock_add_address_msg(cw721_addr.to_string()),
            &[],
        )
        .unwrap();
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(address_list_addr),
            &mock_add_address_msg(buyer.to_string()),
            &[],
        )
        .unwrap();

    // Send Token to Marketplace
    let send_nft_msg = mock_send_nft(
        AndrAddr::from_string(marketplace_addr.clone()),
        token_id.to_string(),
        to_json_binary(&mock_start_sale(Uint128::from(100u128), "uandr", false)).unwrap(),
    );
    router
        .execute_contract(
            owner,
            Addr::unchecked(cw721_addr.clone()),
            &send_nft_msg,
            &[],
        )
        .unwrap();

    // Buy Token
    let buy_msg = mock_buy_token(cw721_addr.clone(), token_id);
    let amp_msg = AMPMsg::new(
        Addr::unchecked(marketplace_addr.clone()),
        to_json_binary(&buy_msg).unwrap(),
        Some(vec![coin(200, "uandr")]),
    );

    let packet = AMPPkt::new(
        buyer.clone(),
        andr.kernel_address.to_string(),
        vec![amp_msg],
    );
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
            Addr::unchecked(marketplace_addr),
            &receive_packet_msg,
            &[coin(200, "uandr")],
        )
        .unwrap();

    // Check final state
    let owner_resp: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(cw721_addr, &mock_cw721_owner_of(token_id.to_string(), None))
        .unwrap();
    assert_eq!(owner_resp.owner, buyer.to_string());

    let balance = router
        .wrap()
        .query_balance(rates_receiver, "uandr")
        .unwrap();
    assert_eq!(balance.amount, Uint128::from(100u128));
}

#[test]
fn test_marketplace_app_cw20() {
    let mut router = mock_app();
    let owner = router.api().addr_make("owner");
    let buyer = router.api().addr_make("buyer");
    let rates_receiver = router.api().addr_make("receiver");
    router
        .send_tokens(
            Addr::unchecked("owner"),
            buyer.clone(),
            &[coin(200, "uandr")],
        )
        .unwrap();

    let andr = mock_andromeda(&mut router, owner.clone());

    // Store contract codes
    let cw721_code_id = router.store_code(mock_andromeda_cw721());
    let cw20_code_id = router.store_code(mock_andromeda_cw20());
    let marketplace_code_id = router.store_code(mock_andromeda_marketplace());
    let app_code_id = router.store_code(mock_andromeda_app());
    let rates_code_id = router.store_code(mock_andromeda_rates());
    let address_list_code_id = router.store_code(mock_andromeda_address_list());

    andr.store_code_id(&mut router, "cw721", cw721_code_id);
    andr.store_code_id(&mut router, "cw20", cw20_code_id);
    andr.store_code_id(&mut router, "marketplace", marketplace_code_id);
    andr.store_code_id(&mut router, "rates", rates_code_id);
    andr.store_code_id(&mut router, "address-list", address_list_code_id);
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
        andr.kernel_address.to_string(),
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
    let rates_init_msg = mock_rates_instantiate_msg(rates, andr.kernel_address.to_string(), None);
    let rates_component =
        AppComponent::new("rates", "rates", to_json_binary(&rates_init_msg).unwrap());

    let address_list_init_msg =
        mock_address_list_instantiate_msg(true, andr.kernel_address.to_string(), None);
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
        andr.kernel_address.to_string(),
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
    let app_init_msg = mock_app_instantiate_msg(
        "Auction App".to_string(),
        app_components.clone(),
        andr.kernel_address.to_string(),
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

    let cw20_addr: String = router
        .wrap()
        .query_wasm_smart(app_addr.clone(), &mock_get_address_msg(cw20_component.name))
        .unwrap();

    let cw721_addr: String = router
        .wrap()
        .query_wasm_smart(
            app_addr.clone(),
            &mock_get_address_msg(cw721_component.name),
        )
        .unwrap();
    let marketplace_addr: String = router
        .wrap()
        .query_wasm_smart(
            app_addr.clone(),
            &mock_get_address_msg(marketplace_component.name),
        )
        .unwrap();
    let address_list_addr: String = router
        .wrap()
        .query_wasm_smart(app_addr, &mock_get_address_msg(address_list_component.name))
        .unwrap();

    // Mint Tokens
    let mint_msg = mock_quick_mint_msg(1, owner.to_string());
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(cw721_addr.clone()),
            &mint_msg,
            &[],
        )
        .unwrap();

    let token_id = "0";

    // Whitelist
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(address_list_addr.clone()),
            &mock_add_address_msg(cw721_addr.to_string()),
            &[],
        )
        .unwrap();
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(address_list_addr.clone()),
            &mock_add_address_msg(buyer.to_string()),
            &[],
        )
        .unwrap();
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(address_list_addr),
            &mock_add_address_msg(cw20_addr.to_string()),
            &[],
        )
        .unwrap();

    // Send Token to Marketplace
    let send_nft_msg = mock_send_nft(
        AndrAddr::from_string(marketplace_addr.clone()),
        token_id.to_string(),
        to_json_binary(&mock_start_sale(
            Uint128::from(100u128),
            cw20_addr.clone(),
            true,
        ))
        .unwrap(),
    );
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(cw721_addr.clone()),
            &send_nft_msg,
            &[],
        )
        .unwrap();

    // Buy Token
    let hook_msg = Cw20HookMsg::Buy {
        token_id: token_id.to_owned(),
        token_address: cw721_addr.clone(),
    };

    let buy_msg = mock_cw20_send(
        AndrAddr::from_string(marketplace_addr),
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
            Addr::unchecked(cw20_addr.clone()),
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
        .query_wasm_smart(cw721_addr, &mock_cw721_owner_of(token_id.to_string(), None))
        .unwrap();
    assert_eq!(owner_resp.owner, buyer.to_string());

    // The NFT owner sold it for 200, there's also a 50% tax so the owner should receive 100
    let cw20_balance_query = mock_get_cw20_balance(owner);
    let cw20_balance_response: BalanceResponse = router
        .wrap()
        .query_wasm_smart(cw20_addr.clone(), &cw20_balance_query)
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
        .query_wasm_smart(cw20_addr.clone(), &cw20_balance_query)
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
        .query_wasm_smart(cw20_addr, &cw20_balance_query)
        .unwrap();
    assert_eq!(cw20_balance_response.balance, Uint128::new(100));
}
