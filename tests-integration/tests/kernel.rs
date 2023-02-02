use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{
    mock_andromeda_app, mock_app_instantiate_msg, mock_get_components_msg,
};
use andromeda_finance::splitter::{AMPRecipient, AddressPercent};

use andromeda_splitter::mock::{
    mock_andromeda_splitter, mock_splitter_instantiate_msg, mock_splitter_send_kernel_msg,
};
use andromeda_testing::mock::MockAndromeda;
use andromeda_vault::mock::{
    mock_andromeda_vault, mock_vault_deposit_msg, mock_vault_get_balance,
    mock_vault_instantiate_msg,
};

use cosmwasm_std::{coin, coins, to_binary, Addr, Coin, Decimal, Uint128};

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
                [coin(1000, "uandr")].to_vec(),
            )
            .unwrap();
    })
}

fn mock_andromeda(app: &mut App, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

#[test]
fn kernel() {
    let owner = Addr::unchecked("owner");
    let recipient = Addr::unchecked("recipient");

    let mut router = mock_app();
    let andr = mock_andromeda(&mut router, owner.clone());

    // Store contract codes
    let vault_code_id = router.store_code(mock_andromeda_vault());
    let splitter_code_id = router.store_code(mock_andromeda_splitter());
    let app_code_id = router.store_code(mock_andromeda_app());

    andr.store_code_id(&mut router, "splitter", splitter_code_id);
    andr.store_code_id(&mut router, "vault", vault_code_id);
    andr.store_code_id(&mut router, "app", app_code_id);

    // Generate Vault Contract
    let vault_init_msg = mock_vault_instantiate_msg();
    let vault_app_component =
        AppComponent::new("vault", "vault", to_binary(&vault_init_msg).unwrap());

    // Generate Splitter Contract
    let vault_deposit_message =
        mock_vault_deposit_msg(Some(AMPRecipient::Addr(recipient.to_string())), None, None);
    let recipients: Vec<AddressPercent> = vec![AddressPercent {
        recipient: AMPRecipient::ado(
            "/am/app1/vault",
            Some(to_binary(&vault_deposit_message).unwrap()),
        ),
        percent: Decimal::percent(100),
    }];

    let splitter_init_msg =
        mock_splitter_instantiate_msg(recipients, andr.kernel_address.clone(), None);
    let splitter_app_component = AppComponent::new(
        "splitter",
        "splitter",
        to_binary(&splitter_init_msg).unwrap(),
    );

    let app_components: Vec<AppComponent> = vec![vault_app_component, splitter_app_component];
    let app_init_msg = mock_app_instantiate_msg(
        "app1",
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

    let splitter_addr = andr.vfs_resolve_path(&mut router, "/am/app1/splitter");
    let vault_addr = andr.vfs_resolve_path(&mut router, "/am/app1/vault");

    let send_msg = mock_splitter_send_kernel_msg(None, None);
    router
        .execute_contract(owner, splitter_addr, &send_msg, &coins(100, "uandr"))
        .unwrap();

    let query_balance =
        mock_vault_get_balance(recipient.to_string(), Some("uandr".to_string()), None);

    let resp: Vec<Coin> = router
        .wrap()
        .query_wasm_smart(vault_addr, &query_balance)
        .unwrap();

    assert!(resp.first().is_some());
    assert_eq!(resp.first().unwrap().amount, Uint128::from(100u128))
}
