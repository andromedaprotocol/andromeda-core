use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{
    mock_andromeda_app, mock_app_instantiate_msg, mock_get_address_msg, mock_get_components_msg,
};
use andromeda_cw721::mock::{
    mock_andromeda_cw721, mock_create_transfer_agreement_msg, mock_cw721_instantiate_msg,
    mock_cw721_owner_of, mock_quick_mint_msg, mock_send_nft, mock_transfer_agreement,
    mock_transfer_nft,
};
use andromeda_non_fungible_tokens::wrapped_cw721::{Cw721Specification, InstantiateType};
use andromeda_testing::mock::MockAndromeda;
use andromeda_wrapped_cw721::mock::{
    mock_andromeda_wrapped_cw721, mock_get_wrapped_cw721_sub_address, mock_unwrap_nft_msg,
    mock_wrap_nft_msg, mock_wrapped_cw721_instantiate_msg,
};
use common::primitive::Value;
use cosmwasm_std::{coin, to_json_binary, Addr};
use cw721::OwnerOfResponse;
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
                [coin(100, "uandr")].to_vec(),
            )
            .unwrap();
    })
}

fn mock_andromeda(app: &mut App, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

#[test]
fn test_wrapped_cw721_app() {
    let owner = Addr::unchecked("owner");
    let buyer = Addr::unchecked("buyer");

    let mut router = mock_app();
    let andr = mock_andromeda(&mut router, owner.clone());

    // Store contract codes
    let cw721_code_id = router.store_code(mock_andromeda_cw721());
    let wrapped_cw721_code_id = router.store_code(mock_andromeda_wrapped_cw721());
    let app_code_id = router.store_code(mock_andromeda_app());
    andr.store_code_id(&mut router, "cw721", cw721_code_id);
    andr.store_code_id(&mut router, "wrapped-cw721", wrapped_cw721_code_id);
    andr.store_code_id(&mut router, "app", app_code_id);

    // Generate App Components
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        owner.to_string(), // Crowdfund must be minter
        None,
        Some(andr.kernel_address.to_string()),
    );
    let cw721_component = AppComponent::new(
        "1".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    let wrapped_cw721_init_msg = mock_wrapped_cw721_instantiate_msg(
        andr.registry_address.to_string(),
        InstantiateType::New(Cw721Specification {
            name: "Test Tokens 2".to_string(),
            symbol: "TT2".to_string(),
            modules: None,
        }),
        true,
        Some(andr.kernel_address.to_string()),
    );
    let wrapped_cw721_component = AppComponent::new(
        "2".to_string(),
        "wrapped-cw721".to_string(),
        to_json_binary(&wrapped_cw721_init_msg).unwrap(),
    );

    let app_components = vec![cw721_component.clone(), wrapped_cw721_component.clone()];
    let app_init_msg = mock_app_instantiate_msg(
        "Wrapped CW721 App".to_string(),
        app_components.clone(),
        andr.kernel_address.to_string(),
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

    // Get Component Addresses
    let cw721_addr: String = router
        .wrap()
        .query_wasm_smart(
            app_addr.clone(),
            &mock_get_address_msg(cw721_component.name),
        )
        .unwrap();
    let wrapped_cw721_addr: String = router
        .wrap()
        .query_wasm_smart(
            app_addr,
            &mock_get_address_msg(wrapped_cw721_component.name),
        )
        .unwrap();
    let wrapped_sub_cw721_addr: String = router
        .wrap()
        .query_wasm_smart(
            wrapped_cw721_addr.to_string(),
            &mock_get_wrapped_cw721_sub_address(),
        )
        .unwrap();

    // Mint Token
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

    // Wrap Token
    let send_msg = mock_send_nft(
        wrapped_cw721_addr.clone(),
        token_id.to_string(),
        to_json_binary(&mock_wrap_nft_msg(Some(token_id.to_string()))).unwrap(),
    );
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(cw721_addr.clone()),
            &send_msg,
            &[],
        )
        .unwrap();

    // Create Transfer Agreement
    let agreement_amount = coin(100, "uandr");
    let xfer_agreement_msg = mock_create_transfer_agreement_msg(
        token_id.to_string(),
        Some(mock_transfer_agreement(
            Value::Raw(agreement_amount.clone()),
            buyer.to_string(),
        )),
    );
    router
        .execute_contract(
            owner,
            Addr::unchecked(wrapped_sub_cw721_addr.clone()),
            &xfer_agreement_msg,
            &[],
        )
        .unwrap();

    // Buy Token
    let xfer_msg = mock_transfer_nft(buyer.to_string(), token_id.to_string());
    router
        .execute_contract(
            buyer.clone(),
            Addr::unchecked(wrapped_sub_cw721_addr.clone()),
            &xfer_msg,
            &[agreement_amount],
        )
        .unwrap();

    // Unwrap Token
    let unwrap_msg = mock_send_nft(
        wrapped_cw721_addr,
        token_id.to_string(),
        to_json_binary(&mock_unwrap_nft_msg()).unwrap(),
    );
    router
        .execute_contract(
            buyer.clone(),
            Addr::unchecked(wrapped_sub_cw721_addr),
            &unwrap_msg,
            &[],
        )
        .unwrap();

    // Check ownership
    let owner_of_query = mock_cw721_owner_of(token_id.to_string(), None);
    let owner: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(cw721_addr, &owner_of_query)
        .unwrap();

    assert_eq!(owner.owner, buyer.to_string())
}
