#![cfg(not(target_arch = "wasm32"))]

use andromeda_address_list::mock::{
    mock_address_list_instantiate_msg, mock_andromeda_address_list, MockAddressList,
};
use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, MockApp};
use andromeda_cw721::mock::{mock_andromeda_cw721, mock_cw721_instantiate_msg, MockCW721};
use andromeda_marketplace::mock::{
    mock_andromeda_marketplace, mock_buy_token, mock_marketplace_instantiate_msg,
    mock_receive_packet, mock_start_sale, MockMarketplace,
};

use andromeda_rates::mock::{mock_andromeda_rates, mock_rates_instantiate_msg};
use andromeda_std::ado_base::rates::{LocalRate, LocalRateType, LocalRateValue, Rate};
use andromeda_std::amp::messages::{AMPMsg, AMPPkt};
use andromeda_std::amp::{AndrAddr, Recipient};
use andromeda_testing::{MockAndromeda, MockContract};
use cosmwasm_std::{coin, to_json_binary, Addr, Uint128};
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
                &Addr::unchecked("buyer"),
                [coin(200, "uandr")].to_vec(),
            )
            .unwrap();
    })
}

fn mock_andromeda(app: &mut App, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

#[test]
fn test_marketplace_app() {
    let owner = Addr::unchecked("owner");
    let buyer = Addr::unchecked("buyer");
    let rates_receiver = Addr::unchecked("receiver");

    let mut router = mock_app();
    let andr = mock_andromeda(&mut router, owner.clone());

    // Store contract codes
    andr.store_ado(&mut router, mock_andromeda_cw721(), "cw721");
    andr.store_ado(&mut router, mock_andromeda_marketplace(), "marketplace");
    andr.store_ado(&mut router, mock_andromeda_rates(), "rates");
    andr.store_ado(&mut router, mock_andromeda_address_list(), "address-list");
    let app_code_id = andr.store_ado(&mut router, mock_andromeda_app(), "app");

    // Generate App Components
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        owner.to_string(),
        andr.kernel.addr().to_string(),
        None,
    );
    let cw721_component = AppComponent::new(
        "1".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );
    let rate = LocalRate {
        rate_type: LocalRateType::Additive,
        recipients: vec![Recipient {
            address: AndrAddr::from_string(rates_receiver.to_string()),
            msg: None,
            ibc_recovery_address: None,
        }],
        value: LocalRateValue::Flat(coin(100_u128, "uandr")),
        description: None,
    };
    let rates_init_msg = mock_rates_instantiate_msg(
        "marketplace".to_string(),
        rate.clone(),
        andr.kernel.addr().to_string(),
        None,
    );
    let rates_component = AppComponent::new("2", "rates", to_json_binary(&rates_init_msg).unwrap());

    let address_list_init_msg =
        mock_address_list_instantiate_msg(true, andr.kernel.addr().to_string(), None);
    let address_list_component = AppComponent::new(
        "3",
        "address-list",
        to_json_binary(&address_list_init_msg).unwrap(),
    );

    let marketplace_init_msg =
        mock_marketplace_instantiate_msg(andr.kernel.addr().to_string(), None);
    let marketplace_component = AppComponent::new(
        "4".to_string(),
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
    let app = MockApp::instantiate(
        app_code_id,
        owner.clone(),
        &mut router,
        "Auction App",
        app_components.clone(),
        andr.kernel.addr(),
        None,
    );

    let components = app.query_components(&router);
    assert_eq!(components, app_components);

    // Claim Ownership
    app.execute_claim_ownership(&mut router, owner.clone(), None)
        .unwrap();

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

    marketplace
        .execute_set_rate(&mut router, owner.clone(), "marketplace", Rate::Local(rate))
        .unwrap();

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
            owner,
            marketplace.addr().clone(),
            token_id,
            &mock_start_sale(Uint128::from(100u128), "uandr"),
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
