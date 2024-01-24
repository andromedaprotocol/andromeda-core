use andromeda_app::app::{AppComponent, ComponentType};
use andromeda_app_contract::mock::{mock_andromeda_app, MockApp};
use andromeda_crowdfund::mock::{
    mock_andromeda_crowdfund, mock_crowdfund_instantiate_msg, MockCrowdfund,
};
use andromeda_cw721::mock::{mock_andromeda_cw721, mock_cw721_instantiate_msg, MockCW721};
use andromeda_finance::splitter::AddressPercent;
use andromeda_std::amp::{AndrAddr, Recipient};

use andromeda_modules::rates::{Rate, RateInfo};
use andromeda_rates::mock::{mock_andromeda_rates, mock_rates_instantiate_msg};
use andromeda_splitter::mock::{
    mock_andromeda_splitter, mock_splitter_instantiate_msg, mock_splitter_send_msg,
};
use andromeda_std::ado_base::modules::Module;
use std::str::FromStr;

use andromeda_testing::{mock::MockAndromeda, mock_contract::MockContract};
use andromeda_vault::mock::{
    mock_andromeda_vault, mock_vault_deposit_msg, mock_vault_instantiate_msg,
};
use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Decimal, Uint128};
use cw721::Expiration;
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
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked("buyer_three"),
                [coin(100, "uandr")].to_vec(),
            )
            .unwrap();
    })
}

fn mock_andromeda(app: &mut App, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

#[test]
fn test_crowdfund_app() {
    let owner = Addr::unchecked("owner");
    let vault_one_recipient_addr = Addr::unchecked("vault_one_recipient");
    let vault_two_recipient_addr = Addr::unchecked("vault_two_recipient");
    let buyer_one = Addr::unchecked("buyer_one");
    let buyer_two = Addr::unchecked("buyer_two");
    let buyer_three = Addr::unchecked("buyer_three");

    let mut router = mock_app();
    let andr = mock_andromeda(&mut router, owner.clone());

    // Store contract codes
    andr.store_ado(&mut router, mock_andromeda_cw721(), "cw721");
    andr.store_ado(&mut router, mock_andromeda_crowdfund(), "crowdfund");
    andr.store_ado(&mut router, mock_andromeda_vault(), "vault");
    andr.store_ado(&mut router, mock_andromeda_splitter(), "splitter");
    let app_code_id = andr.store_ado(&mut router, mock_andromeda_app(), "app");
    let rates_code_id = andr.store_ado(&mut router, mock_andromeda_rates(), "rates");

    // Generate App Components
    // App component names must be less than 3 characters or longer than 54 characters to force them to be 'invalid' as the MockApi struct used within the CosmWasm App struct only contains those two validation checks
    let rates_recipient = "rates_recipient";
    // Generate rates contract
    let rates: Vec<RateInfo> = [RateInfo {
        rate: Rate::Flat(coin(1, "uandr")),
        is_additive: false,
        recipients: [Recipient::from_string(rates_recipient.to_string())].to_vec(),
        description: Some("Some test rate".to_string()),
    }]
    .to_vec();
    let rates_init_msg = mock_rates_instantiate_msg(rates, andr.kernel_address.to_string(), None);
    let rates_addr = router
        .instantiate_contract(
            rates_code_id,
            owner.clone(),
            &rates_init_msg,
            &[],
            "rates",
            None,
        )
        .unwrap();

    let modules: Vec<Module> = vec![Module::new("rates", rates_addr.to_string(), false)];

    let crowdfund_app_component = AppComponent {
        name: "1".to_string(),
        ado_type: "crowdfund".to_string(),
        component_type: ComponentType::New(
            to_json_binary(&mock_crowdfund_instantiate_msg(
                AndrAddr::from_string("./2".to_string()),
                false,
                Some(modules),
                andr.kernel_address.to_string(),
                None,
            ))
            .unwrap(),
        ),
    };
    let cw721_component = AppComponent {
        name: "2".to_string(),
        ado_type: "cw721".to_string(),
        component_type: ComponentType::new(mock_cw721_instantiate_msg(
            "Test Tokens".to_string(),
            "TT".to_string(),
            "./1", // Crowdfund must be minter
            None,
            andr.kernel_address.to_string(),
            None,
        )),
    };
    let vault_one_app_component = AppComponent {
        name: "3".to_string(),
        ado_type: "vault".to_string(),
        component_type: ComponentType::new(mock_vault_instantiate_msg(
            andr.kernel_address.to_string(),
            None,
        )),
    };
    let vault_two_app_component = AppComponent {
        name: "4".to_string(),
        ado_type: "vault".to_string(),
        component_type: ComponentType::new(mock_vault_instantiate_msg(
            andr.kernel_address.to_string(),
            None,
        )),
    };

    // Create splitter recipient structures
    let vault_one_recipient =
        Recipient::from_string(format!("~/am/app/{}", vault_one_app_component.name)).with_msg(
            mock_vault_deposit_msg(
                Some(AndrAddr::from_string(vault_one_recipient_addr.to_string())),
                None,
            ),
        );
    let vault_two_recipient =
        Recipient::from_string(format!("~/am/app/{}", vault_two_app_component.name)).with_msg(
            mock_vault_deposit_msg(
                Some(AndrAddr::from_string(vault_two_recipient_addr.to_string())),
                None,
            ),
        );

    let splitter_recipients = vec![
        AddressPercent {
            recipient: vault_one_recipient,
            percent: Decimal::from_str("0.5").unwrap(),
        },
        AddressPercent {
            recipient: vault_two_recipient,
            percent: Decimal::from_str("0.5").unwrap(),
        },
    ];

    let splitter_init_msg =
        mock_splitter_instantiate_msg(splitter_recipients, andr.kernel_address.clone(), None, None);
    let splitter_app_component = AppComponent {
        name: "5".to_string(),
        component_type: ComponentType::new(&splitter_init_msg),
        ado_type: "splitter".to_string(),
    };

    let app_components = vec![
        cw721_component.clone(),
        crowdfund_app_component.clone(),
        vault_one_app_component.clone(),
        vault_two_app_component.clone(),
        splitter_app_component.clone(),
    ];

    let app = MockApp::instantiate(
        app_code_id,
        owner.clone(),
        &mut router,
        "app",
        app_components.clone(),
        andr.kernel_address.clone(),
        Some(owner.to_string()),
    );

    let components = app.query_components(&mut router);
    assert_eq!(components, app_components);

    let _vault_one_addr = app.query_component_addr(&mut router, vault_one_app_component.name);
    let _vault_two_addr = app.query_component_addr(&mut router, vault_two_app_component.name);
    app.execute_claim_ownership(&mut router, owner.clone(), None);

    let cw721_contract =
        app.query_ado_by_component_name::<MockCW721>(&mut router, cw721_component.name);
    let crowdfund_contract =
        app.query_ado_by_component_name::<MockCrowdfund>(&mut router, crowdfund_app_component.name);

    let minter = cw721_contract.query_minter(&mut router);
    assert_eq!(minter, crowdfund_contract.addr());

    let minter = cw721_contract.query_minter(&mut router);

    assert_eq!(minter, crowdfund_contract.addr());

    // Mint Tokens
    crowdfund_contract.execute_quick_mint(owner.clone(), &mut router, 5, owner.to_string());

    // Start Sale
    let token_price = coin(100, "uandr");

    let sale_recipient =
        Recipient::from_string(format!("~/am/app/{}", splitter_app_component.name))
            .with_msg(mock_splitter_send_msg());
    let expiration = Expiration::AtHeight(router.block_info().height + 5);
    crowdfund_contract.execute_start_sale(
        owner.clone(),
        &mut router,
        expiration,
        token_price.clone(),
        Uint128::from(3u128),
        Some(1),
        sale_recipient,
    );

    // Buy Tokens
    let buyers = vec![buyer_one, buyer_two, buyer_three];
    for buyer in buyers.clone() {
        crowdfund_contract.execute_purchase(buyer, &mut router, Some(1), &[token_price.clone()]);
    }
    let crowdfund_balance = router
        .wrap()
        .query_balance(crowdfund_contract.addr().clone(), token_price.denom)
        .unwrap();
    assert_eq!(crowdfund_balance.amount, Uint128::from(300u128));

    // End Sale
    let block_info = router.block_info();
    router.set_block(BlockInfo {
        height: block_info.height + 5,
        time: block_info.time,
        chain_id: block_info.chain_id,
    });

    crowdfund_contract.execute_end_sale(owner.clone(), &mut router, None);
    crowdfund_contract.execute_end_sale(owner, &mut router, None);

    // Check final state
    //Check token transfers
    for (i, buyer) in buyers.iter().enumerate() {
        let owner = cw721_contract.query_owner_of(&mut router, i.to_string());
        assert_eq!(owner, buyer.to_string());
    }

    // TODO: FIX VAULT BALANCES
    // //Check vault balances

    // let balance_one: Vec<Coin> = router
    //     .wrap()
    //     .query_wasm_smart(
    //         vault_one_addr,
    //         &mock_vault_get_balance(
    //             AndrAddr::from_string(vault_one_recipient_addr.to_string()),
    //             None,
    //             None,
    //         ),
    //     )
    //     .unwrap();
    // assert!(!balance_one.is_empty());
    // assert_eq!(balance_one[0], coin(148, "uandr"));

    // let balance_two: Vec<Coin> = router
    //     .wrap()
    //     .query_wasm_smart(
    //         vault_two_addr,
    //         &mock_vault_get_balance(
    //             AndrAddr::from_string(vault_two_recipient_addr.to_string()),
    //             None,
    //             None,
    //         ),
    //     )
    //     .unwrap();
    // assert!(!balance_two.is_empty());
    // assert_eq!(balance_two[0], coin(148, "uandr"));
}
