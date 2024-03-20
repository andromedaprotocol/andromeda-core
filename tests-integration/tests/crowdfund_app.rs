use andromeda_app::app::{AppComponent, ComponentType};
use andromeda_app_contract::mock::{
    mock_andromeda_app, mock_app_instantiate_msg, mock_claim_ownership_msg, mock_get_address_msg,
    mock_get_components_msg,
};
use andromeda_crowdfund::mock::{
    mock_andromeda_crowdfund, mock_crowdfund_instantiate_msg, mock_crowdfund_quick_mint_msg,
    mock_end_crowdfund_msg, mock_purchase_msg, mock_start_crowdfund_msg,
};
use andromeda_cw721::mock::{
    mock_andromeda_cw721, mock_cw721_instantiate_msg, mock_cw721_owner_of,
};
use andromeda_finance::splitter::AddressPercent;
use andromeda_std::amp::{AndrAddr, Recipient};

use andromeda_modules::rates::{Rate, RateInfo};
use andromeda_rates::mock::{mock_andromeda_rates, mock_rates_instantiate_msg};
use andromeda_splitter::mock::{
    mock_andromeda_splitter, mock_splitter_instantiate_msg, mock_splitter_send_msg,
};
use andromeda_std::ado_base::modules::Module;
use std::str::FromStr;

use andromeda_testing::mock::MockAndromeda;
use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Decimal, Uint128};
use cw721::{Expiration, OwnerOfResponse};
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

// TODO: Fix to check wallet balance post sale
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
    let cw721_code_id = router.store_code(mock_andromeda_cw721());
    let crowdfund_code_id = router.store_code(mock_andromeda_crowdfund());
    let splitter_code_id = router.store_code(mock_andromeda_splitter());
    let app_code_id = router.store_code(mock_andromeda_app());
    let rates_code_id = router.store_code(mock_andromeda_rates());

    andr.store_code_id(&mut router, "cw721", cw721_code_id);
    andr.store_code_id(&mut router, "crowdfund", crowdfund_code_id);
    andr.store_code_id(&mut router, "splitter", splitter_code_id);
    andr.store_code_id(&mut router, "app-contract", app_code_id);
    andr.store_code_id(&mut router, "rates", rates_code_id);

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

    let crowdfund_init_msg = mock_crowdfund_instantiate_msg(
        AndrAddr::from_string("./tokens".to_string()),
        false,
        Some(modules),
        andr.kernel_address.to_string(),
        None,
    );
    let crowdfund_app_component = AppComponent {
        name: "crowdfund".to_string(),
        ado_type: "crowdfund".to_string(),
        component_type: ComponentType::New(to_json_binary(&crowdfund_init_msg).unwrap()),
    };

    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        "./crowdfund", // Crowdfund must be minter
        None,
        andr.kernel_address.to_string(),
        None,
    );
    let cw721_component = AppComponent {
        name: "tokens".to_string(),
        ado_type: "cw721".to_string(),
        component_type: ComponentType::new(cw721_init_msg),
    };

    // The vault query works only for the last element in this vector.
    // Currently the balance check for vault one is failing. But if the elements are switched, it starts working and vault two balance check fails
    // Only one of the recipients' deposit messages is being sent, and it's always the last elements'
    // Eventhough it shows in execute_send's response in the splitter that both messages are being sent.
    let splitter_recipients = vec![
        AddressPercent {
            recipient: Recipient::from_string(vault_one_recipient_addr),
            percent: Decimal::from_str("0.5").unwrap(),
        },
        AddressPercent {
            recipient: Recipient::from_string(vault_two_recipient_addr),
            percent: Decimal::from_str("0.5").unwrap(),
        },
    ];

    let splitter_init_msg =
        mock_splitter_instantiate_msg(splitter_recipients, andr.kernel_address.clone(), None, None);
    let splitter_app_component = AppComponent {
        name: "split".to_string(),
        component_type: ComponentType::new(splitter_init_msg),
        ado_type: "splitter".to_string(),
    };

    let app_components = vec![
        cw721_component.clone(),
        crowdfund_app_component.clone(),
        splitter_app_component.clone(),
    ];
    let app_init_msg = mock_app_instantiate_msg(
        "app".to_string(),
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
            "Crowdfund App",
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
            app_addr.clone(),
            &mock_claim_ownership_msg(None),
            &[],
        )
        .unwrap();

    let crowdfund_addr: String = router
        .wrap()
        .query_wasm_smart(
            app_addr.clone(),
            &mock_get_address_msg(crowdfund_app_component.name),
        )
        .unwrap();

    // Mint Tokens
    let mint_msg = mock_crowdfund_quick_mint_msg(5, owner.to_string());
    andr.accept_ownership(&mut router, crowdfund_addr.clone(), owner.clone());
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(crowdfund_addr.clone()),
            &mint_msg,
            &[],
        )
        .unwrap();

    // Start Sale
    let token_price = coin(100, "uandr");

    let sale_recipient = Recipient::from_string(format!("~am/app/{}", splitter_app_component.name))
        .with_msg(mock_splitter_send_msg());
    let start_msg = mock_start_crowdfund_msg(
        Expiration::AtHeight(router.block_info().height + 5),
        token_price.clone(),
        Uint128::from(3u128),
        Some(1),
        sale_recipient,
    );
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(crowdfund_addr.clone()),
            &start_msg,
            &[],
        )
        .unwrap();

    // Buy Tokens
    let buyers = vec![buyer_one, buyer_two, buyer_three];
    for buyer in buyers.clone() {
        let purchase_msg = mock_purchase_msg(Some(1));
        router
            .execute_contract(
                buyer,
                Addr::unchecked(crowdfund_addr.clone()),
                &purchase_msg,
                &[token_price.clone()],
            )
            .unwrap();
    }
    let crowdfund_balance = router
        .wrap()
        .query_balance(crowdfund_addr.clone(), token_price.denom)
        .unwrap();
    assert_eq!(crowdfund_balance.amount, Uint128::from(300u128));
    // End Sale
    let block_info = router.block_info();
    router.set_block(BlockInfo {
        height: block_info.height + 5,
        time: block_info.time,
        chain_id: block_info.chain_id,
    });
    let end_sale_msg = mock_end_crowdfund_msg(None);
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(crowdfund_addr.clone()),
            &end_sale_msg,
            &[],
        )
        .unwrap();
    router
        .execute_contract(owner, Addr::unchecked(crowdfund_addr), &end_sale_msg, &[])
        .unwrap();

    // Check final state
    //Check token transfers
    let cw721_addr: String = router
        .wrap()
        .query_wasm_smart(app_addr, &mock_get_address_msg(cw721_component.name))
        .unwrap();
    for (i, buyer) in buyers.iter().enumerate() {
        let query_msg = mock_cw721_owner_of(i.to_string(), None);
        let owner: OwnerOfResponse = router
            .wrap()
            .query_wasm_smart(cw721_addr.clone(), &query_msg)
            .unwrap();

        assert_eq!(owner.owner, buyer.to_string());
    }
}
