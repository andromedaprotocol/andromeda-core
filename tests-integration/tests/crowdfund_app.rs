use std::str::FromStr;

use andromeda_adodb::mock::{
    mock_adodb_instantiate_msg, mock_andromeda_adodb, mock_store_code_id_msg,
};
use andromeda_app::app::AppComponent;
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
use andromeda_finance::splitter::{ADORecipient, AMPRecipient, AddressPercent};
use andromeda_kernel::mock::{
    mock_andromeda_kernel, mock_kernel_instantiate_message, mock_upsert_key_address,
};
use andromeda_splitter::mock::{
    mock_andromeda_splitter, mock_splitter_instantiate_msg, mock_splitter_send_msg,
};

use andromeda_testing::mock::MockAndromeda;
use andromeda_vault::mock::{
    mock_andromeda_vault, mock_vault_deposit_msg, mock_vault_get_balance,
    mock_vault_instantiate_msg,
};
use cosmwasm_std::{coin, to_binary, Addr, BlockInfo, Coin, Decimal, Uint128};
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
    let vault_code_id = router.store_code(mock_andromeda_vault());
    let splitter_code_id = router.store_code(mock_andromeda_splitter());
    let app_code_id = router.store_code(mock_andromeda_app());
    let kernel_code_id = router.store_code(mock_andromeda_kernel());
    let adodb_code_id = router.store_code(mock_andromeda_adodb());
    andr.store_code_id(&mut router, "cw721", cw721_code_id);
    andr.store_code_id(&mut router, "crowdfund", crowdfund_code_id);
    andr.store_code_id(&mut router, "vault", vault_code_id);
    andr.store_code_id(&mut router, "splitter", splitter_code_id);
    andr.store_code_id(&mut router, "app", app_code_id);
    andr.store_code_id(&mut router, "kernel", kernel_code_id);

    // Generate App Components
    // App component names must be less than 3 characters or longer than 54 characters to force them to be 'invalid' as the MockApi struct used within the CosmWasm App struct only contains those two validation checks
    let crowdfund_init_msg = mock_crowdfund_instantiate_msg("2".to_string(), false, None);
    let crowdfund_app_component = AppComponent {
        name: "1".to_string(),
        ado_type: "crowdfund".to_string(),
        instantiate_msg: to_binary(&crowdfund_init_msg).unwrap(),
    };

    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        crowdfund_app_component.clone().name, // Crowdfund must be minter
        None,
    );
    let cw721_component = AppComponent {
        name: "2".to_string(),
        ado_type: "cw721".to_string(),
        instantiate_msg: to_binary(&cw721_init_msg).unwrap(),
    };

    let vault_one_init_msg = mock_vault_instantiate_msg();
    let vault_one_app_component = AppComponent {
        name: "3".to_string(),
        ado_type: "vault".to_string(),
        instantiate_msg: to_binary(&vault_one_init_msg).unwrap(),
    };

    let vault_two_init_msg = mock_vault_instantiate_msg();
    let vault_two_app_component = AppComponent {
        name: "4".to_string(),
        ado_type: "vault".to_string(),
        instantiate_msg: to_binary(&vault_two_init_msg).unwrap(),
    };

    let kernel_init_msg = mock_kernel_instantiate_message();

    // Create splitter recipient structures
    let vault_one_recipient = AMPRecipient::ADO(ADORecipient {
        address: "contract8".to_string(),
        msg: Some(
            to_binary(&mock_vault_deposit_msg(
                Some(AMPRecipient::Addr(vault_one_recipient_addr.to_string())),
                None,
                None,
            ))
            .unwrap(),
        ),
    });
    let vault_two_recipient = AMPRecipient::ADO(ADORecipient {
        address: "contract9".to_string(),

        msg: Some(
            to_binary(&mock_vault_deposit_msg(
                Some(AMPRecipient::Addr(vault_two_recipient_addr.to_string())),
                None,
                None,
            ))
            .unwrap(),
        ),
    });
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

    // Instantiate the kernel contract
    let kernel_addr = router
        .instantiate_contract(
            kernel_code_id,
            owner.clone(),
            &kernel_init_msg,
            &[],
            "Kernel",
            Some(owner.to_string()),
        )
        .unwrap();

    print!("Kernel address: {:?}", kernel_addr.to_string());

    let adodb_init_msg = mock_adodb_instantiate_msg();

    // Instantiate the adodb contract
    let adodb_addr = router
        .instantiate_contract(
            adodb_code_id,
            owner.clone(),
            &adodb_init_msg,
            &[],
            "adodb",
            Some(owner.to_string()),
        )
        .unwrap();

    print!("adodb address: {:?}", adodb_addr.to_string());

    // Add the crowdfund's code id into the adodb
    router
        .execute_contract(
            owner.clone(),
            adodb_addr.clone(),
            &mock_store_code_id_msg("crowdfund".to_string(), crowdfund_code_id),
            &[],
        )
        .unwrap();

    // Upsert adodb address in kernel
    router
        .execute_contract(
            owner.clone(),
            kernel_addr.clone(),
            &mock_upsert_key_address("adodb", adodb_addr.clone()),
            &[],
        )
        .unwrap();

    let splitter_init_msg = mock_splitter_instantiate_msg(splitter_recipients, kernel_addr, None);
    let splitter_app_component = AppComponent {
        name: "5".to_string(),
        instantiate_msg: to_binary(&splitter_init_msg).unwrap(),
        ado_type: "splitter".to_string(),
    };

    let app_components = vec![
        cw721_component.clone(),
        crowdfund_app_component.clone(),
        vault_one_app_component.clone(),
        vault_two_app_component.clone(),
        splitter_app_component.clone(),
    ];
    let app_init_msg = mock_app_instantiate_msg(
        "Crowdfund App".to_string(),
        app_components.clone(),
        andr.registry_address.to_string(),
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

    let vault_one_addr: String = router
        .wrap()
        .query_wasm_smart(
            app_addr.clone(),
            &mock_get_address_msg(vault_one_app_component.name),
        )
        .unwrap();
    println!("Vault one address: {vault_one_addr:?}");

    let vault_two_addr: String = router
        .wrap()
        .query_wasm_smart(
            app_addr.clone(),
            &mock_get_address_msg(vault_two_app_component.name),
        )
        .unwrap();
    println!("Vault two address: {vault_two_addr:?}");

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

    // Add the vault's code id into the adodb
    router
        .execute_contract(
            owner.clone(),
            adodb_addr.clone(),
            &mock_store_code_id_msg("vault".to_string(), vault_code_id),
            &[],
        )
        .unwrap();

    let splitter_addr: String = router
        .wrap()
        .query_wasm_smart(
            app_addr.clone(),
            &mock_get_address_msg(splitter_app_component.name),
        )
        .unwrap();
    println!("Splitter address is: {splitter_addr:?}");

    // Add the splitter's code id into the adodb
    router
        .execute_contract(
            owner.clone(),
            adodb_addr,
            &mock_store_code_id_msg("splitter".to_string(), splitter_code_id),
            &[],
        )
        .unwrap();

    // Mint Tokens
    let mint_msg = mock_crowdfund_quick_mint_msg(5, owner.to_string());
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
    let sale_recipient = AMPRecipient::ADO(ADORecipient {
        address: "contract10".to_string(),
        msg: Some(to_binary(&mock_splitter_send_msg()).unwrap()),
    });
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
    let result = router
        .execute_contract(owner, Addr::unchecked(crowdfund_addr), &end_sale_msg, &[])
        .unwrap();
    println!("{result:?}");

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

    //Check vault balances

    let balance_one: Vec<Coin> = router
        .wrap()
        .query_wasm_smart(
            vault_one_addr,
            &mock_vault_get_balance(
                vault_one_recipient_addr.to_string(),
                Some("uandr".to_string()),
                None,
            ),
        )
        .unwrap();
    assert!(!balance_one.is_empty());
    assert_eq!(balance_one[0], coin(150, "uandr"));

    let balance_two: Vec<Coin> = router
        .wrap()
        .query_wasm_smart(
            vault_two_addr,
            &mock_vault_get_balance(
                vault_two_recipient_addr.to_string(),
                Some("uandr".to_string()),
                None,
            ),
        )
        .unwrap();
    assert!(!balance_two.is_empty());
    assert_eq!(balance_two[0], coin(150, "uandr"));
}
