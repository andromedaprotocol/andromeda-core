use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{
    mock_andromeda_app, mock_app_instantiate_msg, mock_get_address_msg, mock_get_components_msg,
};
use andromeda_cw20::mock::{
    mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_cw20_send, mock_cw20_transfer,
    mock_get_cw20_balance, mock_minter,
};
use andromeda_cw20_staking::mock::{
    mock_andromeda_cw20_staking, mock_cw20_get_staker, mock_cw20_stake,
    mock_cw20_staking_instantiate_msg,
};
use andromeda_fungible_tokens::cw20_staking::StakerResponse;
use andromeda_testing::mock::MockAndromeda;
use cosmwasm_std::{coin, to_binary, Addr, Uint128};
use cw20::{BalanceResponse, Cw20Coin};
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
                &Addr::unchecked("staker_one"),
                [coin(100, "uandr")].to_vec(),
            )
            .unwrap();
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked("staker_two"),
                [coin(100, "uandr")].to_vec(),
            )
            .unwrap();
    })
}

fn mock_andromeda(app: &mut App, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

#[test]
fn test_cw20_staking_app() {
    let owner = Addr::unchecked("owner");
    let staker_one = Addr::unchecked("staker_one");
    let staker_two = Addr::unchecked("staker_two");

    let mut router = mock_app();
    let andr = mock_andromeda(&mut router, owner.clone());

    // Store contract codes
    let cw20_code_id = router.store_code(mock_andromeda_cw20());
    let cw20_staking_code_id = router.store_code(mock_andromeda_cw20_staking());
    let app_code_id = router.store_code(mock_andromeda_app());
    andr.store_code_id(&mut router, "cw20", cw20_code_id);
    andr.store_code_id(&mut router, "cw20-staking", cw20_staking_code_id);
    andr.store_code_id(&mut router, "app", app_code_id);

    // Create App Components
    let initial_balances = vec![
        Cw20Coin {
            address: staker_one.to_string(),
            amount: Uint128::from(1000u128),
        },
        Cw20Coin {
            address: staker_two.to_string(),
            amount: Uint128::from(2000u128),
        },
        Cw20Coin {
            address: owner.to_string(),
            amount: Uint128::from(10000u128),
        },
    ];
    let cw20_init_msg = mock_cw20_instantiate_msg(
        "Test Tokens".to_string(),
        "TTT".to_string(),
        6,
        initial_balances,
        Some(mock_minter(
            "owner".to_string(),
            Some(Uint128::from(1000000u128)),
        )),
        None,
    );
    let cw20_component = AppComponent::new(
        "1".to_string(),
        "cw20".to_string(),
        to_binary(&cw20_init_msg).unwrap(),
    );

    let cw20_staking_init_msg = mock_cw20_staking_instantiate_msg(cw20_component.clone().name);
    let cw20_staking_component = AppComponent::new(
        "2".to_string(),
        "cw20-staking".to_string(),
        to_binary(&cw20_staking_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![cw20_component.clone(), cw20_staking_component.clone()];
    let app_init_msg = mock_app_instantiate_msg(
        "Staking App".to_string(),
        app_components.clone(),
        andr.registry_address.to_string(),
    );

    let app_addr = router
        .instantiate_contract(
            app_code_id,
            owner.clone(),
            &app_init_msg,
            &[],
            "Staking App",
            Some(owner.to_string()),
        )
        .unwrap();

    let components: Vec<AppComponent> = router
        .wrap()
        .query_wasm_smart(app_addr.clone(), &mock_get_components_msg())
        .unwrap();

    assert_eq!(components, app_components);

    // Component Addresses
    let cw20_addr: String = router
        .wrap()
        .query_wasm_smart(
            app_addr.to_string(),
            &mock_get_address_msg(cw20_component.name),
        )
        .unwrap();
    let cw20_staking_addr: String = router
        .wrap()
        .query_wasm_smart(
            app_addr.to_string(),
            &mock_get_address_msg(cw20_staking_component.name),
        )
        .unwrap();

    // Check Balances
    let balance_one: BalanceResponse = router
        .wrap()
        .query_wasm_smart(
            cw20_addr.clone(),
            &mock_get_cw20_balance(staker_one.to_string()),
        )
        .unwrap();
    assert_eq!(balance_one.balance, Uint128::from(1000u128));
    let balance_two: BalanceResponse = router
        .wrap()
        .query_wasm_smart(
            cw20_addr.clone(),
            &mock_get_cw20_balance(staker_two.to_string()),
        )
        .unwrap();
    assert_eq!(balance_two.balance, Uint128::from(2000u128));

    // Stake Tokens
    let staking_msg_one = mock_cw20_send(
        cw20_staking_addr.clone(),
        Uint128::from(1000u128),
        to_binary(&mock_cw20_stake()).unwrap(),
    );
    router
        .execute_contract(
            staker_one.clone(),
            Addr::unchecked(cw20_addr.clone()),
            &staking_msg_one,
            &[],
        )
        .unwrap();

    let staking_msg_two = mock_cw20_send(
        cw20_staking_addr.clone(),
        Uint128::from(2000u128),
        to_binary(&mock_cw20_stake()).unwrap(),
    );
    router
        .execute_contract(
            staker_two.clone(),
            Addr::unchecked(cw20_addr.clone()),
            &staking_msg_two,
            &[],
        )
        .unwrap();

    // Transfer Tokens for Reward
    let transfer_msg = mock_cw20_transfer(cw20_staking_addr.clone(), Uint128::from(3000u128));
    router
        .execute_contract(owner, Addr::unchecked(cw20_addr), &transfer_msg, &[])
        .unwrap();

    // Check staking status
    let staker_one_info: StakerResponse = router
        .wrap()
        .query_wasm_smart(
            cw20_staking_addr.clone(),
            &mock_cw20_get_staker(staker_one.to_string()),
        )
        .unwrap();
    assert_eq!(staker_one_info.share, Uint128::from(1000u128));
    assert_eq!(staker_one_info.balance, Uint128::from(2000u128));

    let staker_two_info: StakerResponse = router
        .wrap()
        .query_wasm_smart(
            cw20_staking_addr,
            &mock_cw20_get_staker(staker_two.to_string()),
        )
        .unwrap();
    assert_eq!(staker_two_info.share, Uint128::from(2000u128));
    assert_eq!(staker_two_info.balance, Uint128::from(4000u128));
}
