use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, mock_app_instantiate_msg, MockAppContract};
use andromeda_cw20::mock::{
    mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_cw20_send, mock_get_cw20_balance,
    mock_minter,
};
use andromeda_cw20_redeem::mock::{
    mock_andromeda_cw20_redeem, mock_cw20_redeem_cancel_redemption_condition_msg,
    mock_cw20_redeem_hook_redeem_msg, mock_cw20_redeem_instantiate_msg,
    mock_cw20_redeem_start_redemption_condition_hook_msg,
    mock_cw20_set_redemption_condition_native_msg, mock_get_redemption_condition,
};
use andromeda_fungible_tokens::cw20_redeem::RedemptionResponse;
use andromeda_std::amp::Recipient;
use andromeda_testing::{
    mock::{mock_app, MockAndromeda, MockApp},
    mock_builder::MockAndromedaBuilder,
    MockContract,
};
use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Timestamp, Uint128};
use cw20::{BalanceResponse, Cw20Coin};
use cw_asset::AssetInfo;
use cw_multi_test::Executor;

pub const OWNER_INITIAL_BALANCE: Uint128 = Uint128::new(10_000);

fn setup_andr(router: &mut MockApp) -> MockAndromeda {
    MockAndromedaBuilder::new(router, "admin")
        .with_wallets(vec![
            ("owner", vec![coin(1000, "uandr"), coin(1000, "uusd")]),
            ("user1", vec![]),
            ("user2", vec![]),
        ])
        .with_contracts(vec![
            ("cw20", mock_andromeda_cw20()),
            ("cw20-redeem", mock_andromeda_cw20_redeem()),
            ("app-contract", mock_andromeda_app()),
        ])
        .build(router)
}

fn setup_app(andr: &MockAndromeda, router: &mut MockApp) -> MockAppContract {
    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");
    let user2 = andr.get_wallet("user2");

    // Create App Components
    let initial_balances = vec![
        Cw20Coin {
            address: user2.to_string(),
            amount: Uint128::from(2000u128),
        },
        Cw20Coin {
            address: owner.to_string(),
            amount: OWNER_INITIAL_BALANCE,
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
        andr.kernel.addr().to_string(),
    );
    let cw20_component_1 = AppComponent::new(
        "cw20".to_string(),
        "cw20".to_string(),
        to_json_binary(&cw20_init_msg).unwrap(),
    );

    let initial_balances_2 = vec![
        Cw20Coin {
            address: user1.to_string(),
            amount: Uint128::from(1000u128),
        },
        Cw20Coin {
            address: user2.to_string(),
            amount: Uint128::from(2000u128),
        },
    ];
    let cw20_init_msg = mock_cw20_instantiate_msg(
        None,
        "RDM".to_string(),
        "RDM".to_string(),
        6,
        initial_balances_2,
        Some(mock_minter(
            owner.to_string(),
            Some(Uint128::from(1000000u128)),
        )),
        andr.kernel.addr().to_string(),
    );
    let cw20_component_2 = AppComponent::new(
        "cw20-2".to_string(),
        "cw20".to_string(),
        to_json_binary(&cw20_init_msg).unwrap(),
    );

    let cw20_redeem_init_msg = mock_cw20_redeem_instantiate_msg(
        format!("./{}", cw20_component_2.name),
        andr.kernel.addr().to_string(),
        Some(owner.to_string()),
    );
    let cw20_redeem_component = AppComponent::new(
        "cw20redeem".to_string(),
        "cw20-redeem".to_string(),
        to_json_binary(&cw20_redeem_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![cw20_component_1, cw20_component_2, cw20_redeem_component];
    let app_init_msg = mock_app_instantiate_msg(
        "Redeem App".to_string(),
        app_components,
        andr.kernel.addr().clone(),
        None,
    );

    let app_code_id = andr.get_code_id(router, "app-contract");
    let app = MockAppContract::instantiate(
        app_code_id,
        owner,
        router,
        app_init_msg.name,
        app_init_msg.app_components,
        andr.kernel.addr(),
        None,
    );

    app
}

// Add these helper functions after setup_app but before the tests
fn query_cw20_balance(router: &mut MockApp, token: String, address: String) -> Uint128 {
    let balance: BalanceResponse = router
        .wrap()
        .query_wasm_smart(token, &mock_get_cw20_balance(address))
        .unwrap();
    balance.balance
}

fn query_redemption_condition(router: &mut MockApp, redeem_addr: String) -> RedemptionResponse {
    router
        .wrap()
        .query_wasm_smart(redeem_addr, &mock_get_redemption_condition())
        .unwrap()
}

fn advance_time(router: &mut MockApp, seconds: u64) {
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: Timestamp::from_seconds(router.block_info().time.seconds() + seconds),
        chain_id: router.block_info().chain_id,
    });
}

struct TestAddresses {
    cw20: Addr,
    cw20_2: Addr,
    cw20_redeem: Addr,
}

fn get_addresses(
    router: &mut MockApp,
    andr: &MockAndromeda,
    app: &MockAppContract,
) -> TestAddresses {
    TestAddresses {
        cw20: andr
            .vfs
            .query_resolve_path(router, format!("/home/{}/cw20", app.addr())),
        cw20_2: andr
            .vfs
            .query_resolve_path(router, format!("/home/{}/cw20-2", app.addr())),
        cw20_redeem: andr
            .vfs
            .query_resolve_path(router, format!("/home/{}/cw20redeem", app.addr())),
    }
}

#[test]
fn test_cw20_redeem_app_native() {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    let addresses = get_addresses(&mut router, &andr, &app);
    let cw20_addr_2 = addresses.cw20_2;
    let cw20_redeem_addr = addresses.cw20_redeem;

    // Start native redemption condition
    let start_redemption_condition_msg =
        mock_cw20_set_redemption_condition_native_msg(Uint128::new(2), None, None, None);

    router
        .execute_contract(
            owner.clone(),
            cw20_redeem_addr.clone(),
            &start_redemption_condition_msg,
            &[coin(1000u128, "uandr")],
        )
        .unwrap();

    // Query redemption condition
    let redemption_condition =
        query_redemption_condition(&mut router, cw20_redeem_addr.to_string());

    assert_eq!(
        redemption_condition.redemption.clone().unwrap().asset,
        AssetInfo::Native("uandr".to_string())
    );
    assert_eq!(
        redemption_condition
            .redemption
            .clone()
            .unwrap()
            .exchange_rate,
        Uint128::new(2)
    );
    assert_eq!(
        redemption_condition.redemption.clone().unwrap().amount,
        Uint128::new(1000)
    );
    assert_eq!(
        redemption_condition.redemption.clone().unwrap().recipient,
        Recipient::new(owner.to_string(), None)
    );

    // Let user 1 redeem
    let redeem_msg = mock_cw20_redeem_hook_redeem_msg();
    let send_msg = mock_cw20_send(
        cw20_redeem_addr.clone(),
        Uint128::new(10u128),
        to_json_binary(&redeem_msg).unwrap(),
    );
    // Forward time for the sale to start
    advance_time(&mut router, 51);

    router
        .execute_contract(user1.clone(), cw20_addr_2.clone(), &send_msg, &[])
        .unwrap();

    // Check that the redeemer has received 200 uandr and that the remetion condition recipient received 10 cw20 tokens
    let balance_one: Uint128 =
        query_cw20_balance(&mut router, cw20_addr_2.to_string(), owner.to_string());
    assert_eq!(balance_one, Uint128::from(10u128));

    // Get native balance of user1
    let balance = router.wrap().query_balance(user1.clone(), "uandr").unwrap();
    assert_eq!(balance.amount, Uint128::from(20u128));

    // Test cancel redemption condition

    // Get native balance of owner before canceling
    let balance = router.wrap().query_balance(owner.clone(), "uandr").unwrap();
    assert_eq!(balance.amount, Uint128::zero());

    let cancel_redemption_condition_msg = mock_cw20_redeem_cancel_redemption_condition_msg();
    router
        .execute_contract(
            owner.clone(),
            cw20_redeem_addr.clone(),
            &cancel_redemption_condition_msg,
            &[],
        )
        .unwrap();

    // Get native balance of owner
    let balance = router.wrap().query_balance(owner.clone(), "uandr").unwrap();
    assert_eq!(balance.amount, Uint128::from(980u128));
}

#[test]
fn test_cw20_redeem_app_native_refund() {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    let addresses = get_addresses(&mut router, &andr, &app);
    let cw20_addr_2 = addresses.cw20_2;
    let cw20_redeem_addr = addresses.cw20_redeem;

    // Start native redemption condition
    let start_redemption_condition_msg =
        mock_cw20_set_redemption_condition_native_msg(Uint128::new(2), None, None, None);

    router
        .execute_contract(
            owner.clone(),
            cw20_redeem_addr.clone(),
            &start_redemption_condition_msg,
            &[coin(100u128, "uandr")],
        )
        .unwrap();

    // Let user 1 redeem
    let redeem_msg = mock_cw20_redeem_hook_redeem_msg();
    let send_msg = mock_cw20_send(
        cw20_redeem_addr.clone(),
        Uint128::new(10u128),
        to_json_binary(&redeem_msg).unwrap(),
    );
    // Forward time for the sale to start
    advance_time(&mut router, 51);

    router
        .execute_contract(user1.clone(), cw20_addr_2.clone(), &send_msg, &[])
        .unwrap();

    // Check that the redeemer has received 20 uandr and that the remetion condition recipient received 10 cw20 tokens
    let balance_one: Uint128 =
        query_cw20_balance(&mut router, cw20_addr_2.to_string(), owner.to_string());
    assert_eq!(balance_one, Uint128::from(10u128));

    // Get native balance of user1
    let balance = router.wrap().query_balance(user1.clone(), "uandr").unwrap();
    assert_eq!(balance.amount, Uint128::from(20u128));

    let balance_one: Uint128 =
        query_cw20_balance(&mut router, cw20_addr_2.to_string(), user1.to_string());
    assert_eq!(balance_one, Uint128::from(1000 - 10u128));

    // Test redemption with refund
    let redeem_msg = mock_cw20_redeem_hook_redeem_msg();
    let send_msg = mock_cw20_send(
        cw20_redeem_addr.clone(),
        // 40 gets the max amount, so the user must be refunded 60
        Uint128::new(100u128),
        to_json_binary(&redeem_msg).unwrap(),
    );

    router
        .execute_contract(user1.clone(), cw20_addr_2.clone(), &send_msg, &[])
        .unwrap();

    // Get native balance of user1
    let balance = router.wrap().query_balance(user1.clone(), "uandr").unwrap();
    assert_eq!(balance.amount, Uint128::from(100u128));

    let balance_one: Uint128 =
        query_cw20_balance(&mut router, cw20_addr_2.to_string(), user1.to_string());
    // Eventhough the user sent 100 to be redeemed the second time, only 40 ended up being deducted since any excess over that was refunded
    assert_eq!(balance_one, Uint128::from(1000 - 10u128 - 40));
}

#[test]
fn test_cw20_redeem_app_cw20() {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    // Component Addresses
    let addresses = get_addresses(&mut router, &andr, &app);
    let (cw20_addr, cw20_addr_2, cw20_redeem_addr) =
        (addresses.cw20, addresses.cw20_2, addresses.cw20_redeem);

    // Start cw20 redemption condition
    let start_redemption_condition_msg =
        mock_cw20_redeem_start_redemption_condition_hook_msg(Uint128::new(2), None, None, None);

    let send_msg = mock_cw20_send(
        cw20_redeem_addr.clone(),
        OWNER_INITIAL_BALANCE,
        to_json_binary(&start_redemption_condition_msg).unwrap(),
    );

    router
        .execute_contract(owner.clone(), cw20_addr.clone(), &send_msg, &[])
        .unwrap();

    // Let user 1 redeem
    let redeem_msg = mock_cw20_redeem_hook_redeem_msg();
    let send_msg = mock_cw20_send(
        cw20_redeem_addr.clone(),
        Uint128::new(10u128),
        to_json_binary(&redeem_msg).unwrap(),
    );
    // Forward time for the sale to start
    advance_time(&mut router, 51);

    router
        .execute_contract(user1.clone(), cw20_addr_2.clone(), &send_msg, &[])
        .unwrap();

    // Check that the redeemer has received 200 uandr and that the redemption condition recipient received 10 cw20 tokens
    let balance_one: Uint128 =
        query_cw20_balance(&mut router, cw20_addr_2.to_string(), owner.to_string());
    assert_eq!(balance_one, Uint128::from(10u128));

    // Get cw20 balance of user1
    let balance_one: Uint128 =
        query_cw20_balance(&mut router, cw20_addr.to_string(), user1.to_string());
    assert_eq!(balance_one, Uint128::from(20u128));

    // Get cw20 balance of owner
    let owner_balance: Uint128 =
        query_cw20_balance(&mut router, cw20_addr.to_string(), owner.to_string());
    assert_eq!(owner_balance, Uint128::zero());

    // Get cw20 balance of redeem contract
    let redeem_contract_balance: Uint128 = query_cw20_balance(
        &mut router,
        cw20_addr.to_string(),
        cw20_redeem_addr.to_string(),
    );
    assert_eq!(
        redeem_contract_balance,
        OWNER_INITIAL_BALANCE.checked_sub(Uint128::new(20)).unwrap()
    );

    // Test cancel redemption condition

    // Get cw20 balance of owner before cancelling
    let owner_balance: Uint128 =
        query_cw20_balance(&mut router, cw20_addr.to_string(), owner.to_string());
    assert_eq!(owner_balance, Uint128::zero());
    let cancel_redemption_condition_msg = mock_cw20_redeem_cancel_redemption_condition_msg();
    router
        .execute_contract(
            owner.clone(),
            cw20_redeem_addr.clone(),
            &cancel_redemption_condition_msg,
            &[],
        )
        .unwrap();

    // Get cw20 balance of redeem contract
    let redeem_contract_balance: Uint128 = query_cw20_balance(
        &mut router,
        cw20_addr.to_string(),
        cw20_redeem_addr.to_string(),
    );
    assert_eq!(redeem_contract_balance, Uint128::zero());

    // Get cw20 balance of owner
    let owner_balance: Uint128 =
        query_cw20_balance(&mut router, cw20_addr.to_string(), owner.to_string());
    assert_eq!(
        owner_balance,
        OWNER_INITIAL_BALANCE.checked_sub(Uint128::new(20)).unwrap()
    );
}
