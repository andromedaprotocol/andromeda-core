use std::fs;

use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, mock_app_instantiate_msg, MockAppContract};
use andromeda_cw20::mock::{
    mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_cw20_send, mock_get_cw20_balance,
    mock_get_version, mock_minter,
};
use andromeda_cw20_redeem::mock::mock_andromeda_cw20_redeem;
use andromeda_cw20_redeem::mock::mock_cw20_redeem_hook_redeem_msg;
use andromeda_cw20_redeem::mock::mock_cw20_redeem_instantiate_msg;
use andromeda_cw20_redeem::mock::mock_cw20_redeem_start_redemption_clause_hook_msg;
use andromeda_cw20_redeem::mock::mock_cw20_set_redemption_clause_native_msg;
use andromeda_cw20_redeem::mock::mock_get_redemption_clause;
use andromeda_fungible_tokens::cw20_redeem::RedemptionResponse;

use andromeda_std::ado_base::version::VersionResponse;
use andromeda_testing::{
    mock::{mock_app, MockAndromeda, MockApp},
    mock_builder::MockAndromedaBuilder,
    MockContract,
};
use cosmwasm_std::{coin, to_json_binary, BlockInfo, Timestamp, Uint128};
use cw20::{BalanceResponse, Cw20Coin};
use cw_asset::AssetInfo;
use cw_multi_test::Executor;
use toml::Value;

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
            amount: Uint128::from(10_000u128),
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

fn get_cw20_contract_version() -> Result<String, Box<dyn std::error::Error>> {
    // Read the Cargo.toml file
    let content = fs::read_to_string("../contracts/fungible-tokens/andromeda-cw20/Cargo.toml")?;

    // Parse the Cargo.toml content
    let parsed_toml = content.parse::<Value>()?;

    // Extract the version string
    if let Some(version) = parsed_toml
        .get("package")
        .and_then(|pkg| pkg.get("version"))
        .and_then(|v| v.as_str())
    {
        Ok(version.to_string())
    } else {
        Err("Version not found in Cargo.toml".into())
    }
}

#[test]
fn test_cw20_redeem_app_native() {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    // Component Addresses
    let cw20_addr = andr
        .vfs
        .query_resolve_path(&mut router, format!("/home/{}/cw20", app.addr()));

    let cw20_addr_2 = andr
        .vfs
        .query_resolve_path(&mut router, format!("/home/{}/cw20-2", app.addr()));

    let cw20_redeem_addr = andr
        .vfs
        .query_resolve_path(&mut router, format!("/home/{}/cw20redeem", app.addr()));

    let version: VersionResponse = router
        .wrap()
        .query_wasm_smart(cw20_addr.clone(), &mock_get_version())
        .unwrap();
    assert_eq!(version.version, get_cw20_contract_version().unwrap());

    // Start native redemption clause
    let start_redemption_clause_msg =
        mock_cw20_set_redemption_clause_native_msg(Uint128::new(2), None, None);

    router
        .execute_contract(
            owner.clone(),
            cw20_redeem_addr.clone(),
            &start_redemption_clause_msg,
            &[coin(1000u128, "uandr")],
        )
        .unwrap();

    // Query redemption clause
    let redemption_clause: RedemptionResponse = router
        .wrap()
        .query_wasm_smart(cw20_redeem_addr.clone(), &mock_get_redemption_clause())
        .unwrap();

    assert_eq!(
        redemption_clause.redemption.clone().unwrap().asset,
        AssetInfo::Native("uandr".to_string())
    );
    assert_eq!(
        redemption_clause.redemption.clone().unwrap().exchange_rate,
        Uint128::new(2)
    );
    assert_eq!(
        redemption_clause.redemption.clone().unwrap().amount,
        Uint128::new(1000)
    );
    assert_eq!(
        redemption_clause.redemption.clone().unwrap().recipient,
        owner.to_string()
    );

    // Let user 1 redeem
    let redeem_msg = mock_cw20_redeem_hook_redeem_msg();
    let send_msg = mock_cw20_send(
        cw20_redeem_addr.clone(),
        Uint128::new(10u128),
        to_json_binary(&redeem_msg).unwrap(),
    );
    // Forward time for the sale to start
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: Timestamp::from_seconds(router.block_info().time.seconds() + 51),
        chain_id: router.block_info().chain_id,
    });

    router
        .execute_contract(user1.clone(), cw20_addr_2.clone(), &send_msg, &[])
        .unwrap();

    // Check that the redeemer has received 200 uandr and that the remetion clause recipient received 10 cw20 tokens
    let balance_one: BalanceResponse = router
        .wrap()
        .query_wasm_smart(
            cw20_addr_2.clone(),
            &mock_get_cw20_balance(owner.to_string()),
        )
        .unwrap();
    assert_eq!(balance_one.balance, Uint128::from(10u128));

    // Get native balance of user1
    let balance = router.wrap().query_balance(user1.clone(), "uandr").unwrap();
    assert_eq!(balance.amount, Uint128::from(20u128));
}

#[test]
fn test_cw20_redeem_app_cw20() {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    // Component Addresses
    let cw20_addr = andr
        .vfs
        .query_resolve_path(&mut router, format!("/home/{}/cw20", app.addr()));

    let cw20_addr_2 = andr
        .vfs
        .query_resolve_path(&mut router, format!("/home/{}/cw20-2", app.addr()));

    let cw20_redeem_addr = andr
        .vfs
        .query_resolve_path(&mut router, format!("/home/{}/cw20redeem", app.addr()));

    // Start cw20 redemption clause
    let start_redemption_clause_msg =
        mock_cw20_redeem_start_redemption_clause_hook_msg(Uint128::new(2), None, None);

    let send_msg = mock_cw20_send(
        cw20_redeem_addr.clone(),
        Uint128::new(100u128),
        to_json_binary(&start_redemption_clause_msg).unwrap(),
    );

    router
        .execute_contract(owner.clone(), cw20_addr.clone(), &send_msg, &[])
        .unwrap();

    // Query redemption clause
    let redemption_clause: RedemptionResponse = router
        .wrap()
        .query_wasm_smart(cw20_redeem_addr.clone(), &mock_get_redemption_clause())
        .unwrap();

    assert_eq!(
        redemption_clause.redemption.clone().unwrap().asset,
        AssetInfo::Cw20(cw20_addr.clone())
    );
    assert_eq!(
        redemption_clause.redemption.clone().unwrap().exchange_rate,
        Uint128::new(2)
    );
    assert_eq!(
        redemption_clause.redemption.clone().unwrap().amount,
        Uint128::new(100)
    );
    assert_eq!(
        redemption_clause.redemption.clone().unwrap().recipient,
        owner.to_string()
    );

    // Let user 1 redeem
    let redeem_msg = mock_cw20_redeem_hook_redeem_msg();
    let send_msg = mock_cw20_send(
        cw20_redeem_addr.clone(),
        Uint128::new(10u128),
        to_json_binary(&redeem_msg).unwrap(),
    );
    // Forward time for the sale to start
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: Timestamp::from_seconds(router.block_info().time.seconds() + 51),
        chain_id: router.block_info().chain_id,
    });

    router
        .execute_contract(user1.clone(), cw20_addr_2.clone(), &send_msg, &[])
        .unwrap();

    // Check that the redeemer has received 200 uandr and that the redemption clause recipient received 10 cw20 tokens
    let balance_one: BalanceResponse = router
        .wrap()
        .query_wasm_smart(
            cw20_addr_2.clone(),
            &mock_get_cw20_balance(owner.to_string()),
        )
        .unwrap();
    assert_eq!(balance_one.balance, Uint128::from(10u128));

    // Get cw20 balance of user1
    let balance_one: BalanceResponse = router
        .wrap()
        .query_wasm_smart(cw20_addr.clone(), &mock_get_cw20_balance(user1.to_string()))
        .unwrap();
    assert_eq!(balance_one.balance, Uint128::from(20u128));

    // Get cw20 balance of owner
    let balance_one: BalanceResponse = router
        .wrap()
        .query_wasm_smart(cw20_addr.clone(), &mock_get_cw20_balance(owner.to_string()))
        .unwrap();
    assert_eq!(balance_one.balance, Uint128::from(9900u128));

    // Get cw20 balance of redeem contract
    let balance_one: BalanceResponse = router
        .wrap()
        .query_wasm_smart(
            cw20_addr.clone(),
            &mock_get_cw20_balance(cw20_redeem_addr.to_string()),
        )
        .unwrap();
    assert_eq!(balance_one.balance, Uint128::from(80u128));
}
