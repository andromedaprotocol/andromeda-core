use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, mock_app_instantiate_msg, MockAppContract};
use andromeda_cw20::mock::{
    mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_cw20_send, mock_get_cw20_balance,
    mock_minter,
};
use andromeda_cw20_exchange::mock::{
    mock_andromeda_cw20_exchange, mock_cw20_exchange_hook_purchase_msg,
    mock_cw20_exchange_instantiate_msg, mock_cw20_exchange_start_sale_msg, mock_redeem_cw20_msg,
    mock_redeem_native_msg, mock_redeem_query_msg, mock_replenish_redeem_cw20_msg,
    mock_replenish_redeem_native_msg, mock_set_redeem_condition_native_msg,
    mock_start_redeem_cw20_msg,
};
use andromeda_fungible_tokens::cw20_exchange::RedeemResponse;
use andromeda_std::{
    amp::{AndrAddr, Recipient},
    error::ContractError,
};
use andromeda_testing::{
    mock::{mock_app, MockAndromeda, MockApp},
    mock_builder::MockAndromedaBuilder,
    MockContract,
};
use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Decimal256, Timestamp, Uint128};
use cw20::{BalanceResponse, Cw20Coin};
use cw_asset::AssetInfo;
use cw_multi_test::Executor;

pub const OWNER_INITIAL_BALANCE: Uint128 = Uint128::new(10_000);
pub const USER1_INITIAL_BALANCE: Uint128 = Uint128::new(10);

fn setup_andr(router: &mut MockApp) -> MockAndromeda {
    MockAndromedaBuilder::new(router, "admin")
        .with_wallets(vec![
            ("owner", vec![coin(10000, "uandr"), coin(10000, "uusd")]),
            (
                "user1",
                vec![
                    coin(1000, "uandr"),
                    coin(USER1_INITIAL_BALANCE.u128(), "uusd"),
                ],
            ),
            ("user2", vec![]),
        ])
        .with_contracts(vec![
            ("cw20", mock_andromeda_cw20()),
            ("cw20_exchange", mock_andromeda_cw20_exchange()),
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
            address: user1.to_string(),
            amount: Uint128::from(1000u128),
        },
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
            address: owner.to_string(),
            amount: OWNER_INITIAL_BALANCE,
        },
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

    let cw20_exchange_init_msg = mock_cw20_exchange_instantiate_msg(
        AndrAddr::from_string("./cw20-2"),
        andr.kernel.addr().to_string(),
        Some(owner.to_string()),
    );
    let cw20_exchange_component = AppComponent::new(
        "cw20_exchange".to_string(),
        "cw20_exchange".to_string(),
        to_json_binary(&cw20_exchange_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![cw20_component_1, cw20_component_2, cw20_exchange_component];
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

fn _advance_time(router: &mut MockApp, seconds: u64) {
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: Timestamp::from_seconds(router.block_info().time.seconds() + seconds),
        chain_id: router.block_info().chain_id,
    });
}

struct TestAddresses {
    cw20: Addr,
    cw20_2: Addr,
    cw20_exchange: Addr,
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
        cw20_exchange: andr
            .vfs
            .query_resolve_path(router, format!("/home/{}/cw20_exchange", app.addr())),
    }
}

const ORIGINAL_SALE_AMOUNT: Uint128 = Uint128::new(1000u128);

#[test]
fn test_cw20_exchange_app_cw20_to_native() {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    let addresses = get_addresses(&mut router, &andr, &app);

    let cw20_addr = addresses.cw20;
    let cw20_addr_2 = addresses.cw20_2;
    let cw20_exchange_addr = addresses.cw20_exchange;

    let cw20_addr_2_asset = AssetInfo::Cw20(cw20_addr_2.clone());
    let cw20_redeem_asset = AssetInfo::Cw20(cw20_addr.clone());

    // Sell a cw20
    let start_sale_msg =
        mock_cw20_exchange_start_sale_msg(cw20_redeem_asset, Uint128::new(2), None, None, None);

    let cw20_send_msg = mock_cw20_send(
        cw20_exchange_addr.clone(),
        ORIGINAL_SALE_AMOUNT,
        to_json_binary(&start_sale_msg).unwrap(),
    );

    router
        .execute_contract(owner.clone(), cw20_addr_2.clone(), &cw20_send_msg, &[])
        .unwrap();

    // Now there's a sale for cw20addr2 for 2 cw20addr per token
    // user1 will purchase 10 cw20addr2

    let purchase_msg =
        mock_cw20_exchange_hook_purchase_msg(Some(Recipient::from_string(user1.to_string())));
    let cw20_send_msg = mock_cw20_send(
        cw20_exchange_addr.clone(),
        Uint128::new(10u128),
        to_json_binary(&purchase_msg).unwrap(),
    );

    router
        .execute_contract(user1.clone(), cw20_addr.clone(), &cw20_send_msg, &[])
        .unwrap();

    // Check that user1 has received 5 cw20addr_2
    let balance = query_cw20_balance(&mut router, cw20_addr_2.to_string(), user1.to_string());
    assert_eq!(balance, Uint128::new(1000 + 5u128));

    // Now the owner will setup a redeem condition for 2 uandr per cw20addr
    let redeem_msg = mock_set_redeem_condition_native_msg(
        cw20_addr_2_asset.clone(),
        Decimal256::from_ratio(Uint128::new(2), Uint128::new(1)),
        Some(Recipient::from_string(owner.to_string())),
        None,
        None,
    );
    router
        .execute_contract(
            owner.clone(),
            cw20_exchange_addr.clone(),
            &redeem_msg,
            &[coin(100u128, "uandr")],
        )
        .unwrap();

    let redeem_query_msg = mock_redeem_query_msg(cw20_addr_2_asset.inner().clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(cw20_exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        AssetInfo::Native("uandr".to_string())
    );
    assert_eq!(redeem_query_resp.redeem.unwrap().amount, Uint128::new(100));

    // Now user1 will try to redeem 5 cw20addr_2
    let redeem_msg = mock_redeem_cw20_msg(Some(Recipient::from_string(user1.to_string())));
    let cw20_send_msg = mock_cw20_send(
        cw20_exchange_addr.clone(),
        Uint128::new(5u128),
        to_json_binary(&redeem_msg).unwrap(),
    );

    router
        .execute_contract(user1.clone(), cw20_addr_2.clone(), &cw20_send_msg, &[])
        .unwrap();

    // Check that user1 has received 10 uandr
    let balance = router.wrap().query_balance(user1.clone(), "uandr").unwrap();
    assert_eq!(balance.amount, Uint128::new(1000 + 10u128));

    let redeem_query_msg = mock_redeem_query_msg(cw20_addr_2_asset.inner().clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(cw20_exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        AssetInfo::Native("uandr".to_string())
    );
    assert_eq!(
        redeem_query_resp.redeem.unwrap().amount,
        Uint128::new(100 - 10u128)
    );

    // User1 will now try to redeem 60 cw20addr2, but he should be refunded 10 since the first 50 will deplete the redeemable amount
    let redeem_msg = mock_redeem_cw20_msg(Some(Recipient::from_string(user1.to_string())));
    let cw20_send_msg = mock_cw20_send(
        cw20_exchange_addr.clone(),
        Uint128::new(60u128),
        to_json_binary(&redeem_msg).unwrap(),
    );

    router
        .execute_contract(user1.clone(), cw20_addr_2.clone(), &cw20_send_msg, &[])
        .unwrap();

    // Check that user1 has received 5 uandr
    let balance = router.wrap().query_balance(user1.clone(), "uandr").unwrap();
    // Initial balance is 10, total redeemable is 100, 50 was redeemed, 10 was refunded
    assert_eq!(balance.amount, Uint128::new(1000 + 100));

    // Query user1's cw20addr2 balance
    let balance = query_cw20_balance(&mut router, cw20_addr_2.to_string(), user1.to_string());
    // 1000 is the original balance, 5 for the amount purchased,
    // 5 is the amount sent in the first redeem,
    // 60 is the amount for the second redeem,
    // the last 10 is for the refund
    assert_eq!(balance, Uint128::new(1000 + 5u128 - 60u128 + 10u128));

    let redeem_query_msg = mock_redeem_query_msg(cw20_addr_2_asset.inner().clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(cw20_exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(redeem_query_resp.redeem.unwrap().amount, Uint128::zero());

    // User 1 will try to redeem but there is no redeemable amount left
    let redeem_msg = mock_redeem_cw20_msg(Some(Recipient::from_string(user1.to_string())));
    let cw20_send_msg = mock_cw20_send(
        cw20_exchange_addr.clone(),
        Uint128::new(1u128),
        to_json_binary(&redeem_msg).unwrap(),
    );

    let err: ContractError = router
        .execute_contract(user1.clone(), cw20_addr_2.clone(), &cw20_send_msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::InvalidFunds {
            msg: "Not enough funds sent to purchase a token".to_string()
        }
    );
}

#[test]
fn test_cw20_exchange_app_cw20_to_cw20() {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    let addresses = get_addresses(&mut router, &andr, &app);

    let cw20_addr = addresses.cw20;
    let cw20_addr_2 = addresses.cw20_2;
    let cw20_exchange_addr = addresses.cw20_exchange;

    let cw20_addr_2_asset = AssetInfo::Cw20(cw20_addr_2.clone());
    let cw20_redeem_asset = AssetInfo::Cw20(cw20_addr.clone());

    // Sell a cw20
    let start_sale_msg =
        mock_cw20_exchange_start_sale_msg(cw20_redeem_asset, Uint128::new(2), None, None, None);

    let cw20_send_msg = mock_cw20_send(
        cw20_exchange_addr.clone(),
        ORIGINAL_SALE_AMOUNT,
        to_json_binary(&start_sale_msg).unwrap(),
    );

    router
        .execute_contract(owner.clone(), cw20_addr_2.clone(), &cw20_send_msg, &[])
        .unwrap();

    // Now there's a sale for cw20addr2 for 2 cw20addr per token
    // user1 will purchase 10 cw20addr2

    let purchase_msg =
        mock_cw20_exchange_hook_purchase_msg(Some(Recipient::from_string(user1.to_string())));
    let cw20_send_msg = mock_cw20_send(
        cw20_exchange_addr.clone(),
        Uint128::new(10u128),
        to_json_binary(&purchase_msg).unwrap(),
    );

    router
        .execute_contract(user1.clone(), cw20_addr.clone(), &cw20_send_msg, &[])
        .unwrap();

    // Check that user1 has received 5 cw20addr_2
    let balance = query_cw20_balance(&mut router, cw20_addr_2.to_string(), user1.to_string());
    assert_eq!(balance, Uint128::new(1000 + 5u128));

    // Now the owner will setup a redeem condition for 2 cw20 per cw20addr
    let start_redeem_msg = mock_start_redeem_cw20_msg(
        None,
        cw20_addr_2_asset.clone(),
        Decimal256::from_ratio(Uint128::new(2), Uint128::new(1)),
        None,
        None,
    );

    let cw20_send_msg = mock_cw20_send(
        cw20_exchange_addr.clone(),
        Uint128::new(100u128),
        to_json_binary(&start_redeem_msg).unwrap(),
    );

    router
        .execute_contract(owner.clone(), cw20_addr.clone(), &cw20_send_msg, &[])
        .unwrap();

    let redeem_query_msg = mock_redeem_query_msg(cw20_addr_2_asset.inner().clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(cw20_exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        AssetInfo::Cw20(cw20_addr.clone())
    );
    assert_eq!(redeem_query_resp.redeem.unwrap().amount, Uint128::new(100));

    // Now user1 will try to redeem 5 cw20addr_2
    let redeem_msg = mock_redeem_cw20_msg(Some(Recipient::from_string(user1.to_string())));
    let cw20_send_msg = mock_cw20_send(
        cw20_exchange_addr.clone(),
        Uint128::new(5u128),
        to_json_binary(&redeem_msg).unwrap(),
    );

    router
        .execute_contract(user1.clone(), cw20_addr_2.clone(), &cw20_send_msg, &[])
        .unwrap();

    // 1000 is the original balance, 10 is the amount purchased with previously in the test, 10 is the amount received in the redeem
    let balance = query_cw20_balance(&mut router, cw20_addr.to_string(), user1.to_string());
    assert_eq!(balance, Uint128::new(1000 - 10 + 10u128));

    let redeem_query_msg = mock_redeem_query_msg(cw20_addr_2_asset.inner().clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(cw20_exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        AssetInfo::Cw20(cw20_addr.clone())
    );
    assert_eq!(
        redeem_query_resp.redeem.unwrap().amount,
        Uint128::new(100 - 10u128)
    );

    // User1 will now try to redeem 60 cw20addr2, but he should be refunded 10 since the first 50 will deplete the redeemable amount
    let redeem_msg = mock_redeem_cw20_msg(Some(Recipient::from_string(user1.to_string())));
    let cw20_send_msg = mock_cw20_send(
        cw20_exchange_addr.clone(),
        Uint128::new(60u128),
        to_json_binary(&redeem_msg).unwrap(),
    );

    router
        .execute_contract(user1.clone(), cw20_addr_2.clone(), &cw20_send_msg, &[])
        .unwrap();

    let balance = query_cw20_balance(&mut router, cw20_addr.to_string(), user1.to_string());
    // Redeemed the remaining 90
    assert_eq!(balance, Uint128::new(1000 + 90));

    // Query user1's cw20addr2 balance
    let balance = query_cw20_balance(&mut router, cw20_addr_2.to_string(), user1.to_string());
    // 1000 is the original balance, 5 for the amount purchased,
    // 5 is the amount sent in the first redeem,
    // 60 is the amount for the second redeem,
    // the last 10 is for the refund
    assert_eq!(balance, Uint128::new(1000 + 5u128 - 60u128 + 10u128));

    let redeem_query_msg = mock_redeem_query_msg(cw20_addr_2_asset.inner().clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(cw20_exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(redeem_query_resp.redeem.unwrap().amount, Uint128::zero());

    // User 1 will try to redeem but there is no redeemable amount left
    let redeem_msg = mock_redeem_cw20_msg(Some(Recipient::from_string(user1.to_string())));
    let cw20_send_msg = mock_cw20_send(
        cw20_exchange_addr.clone(),
        Uint128::new(1u128),
        to_json_binary(&redeem_msg).unwrap(),
    );

    let err: ContractError = router
        .execute_contract(user1.clone(), cw20_addr_2.clone(), &cw20_send_msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::InvalidFunds {
            msg: "Not enough funds sent to purchase a token".to_string()
        }
    );

    // Replenish the redeem
    let replenish_msg = mock_replenish_redeem_cw20_msg(cw20_addr_2_asset.clone());
    let cw20_send_msg = mock_cw20_send(
        cw20_exchange_addr.clone(),
        Uint128::new(10u128),
        to_json_binary(&replenish_msg).unwrap(),
    );

    router
        .execute_contract(owner.clone(), cw20_addr.clone(), &cw20_send_msg, &[])
        .unwrap();

    let redeem_query_msg = mock_redeem_query_msg(cw20_addr_2_asset.inner().clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(cw20_exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.unwrap().amount,
        Uint128::new(10u128)
    );
}

#[test]
fn test_cw20_exchange_app_redeem_native_to_native() {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    let addresses = get_addresses(&mut router, &andr, &app);

    let cw20_exchange_addr = addresses.cw20_exchange;
    let uandr_asset = AssetInfo::Native("uandr".to_string());

    // Now the owner will setup a redeem condition for 2 uandr per cw20addr
    let redeem_msg = mock_set_redeem_condition_native_msg(
        uandr_asset.clone(),
        Decimal256::from_ratio(Uint128::new(2), Uint128::new(1)),
        Some(Recipient::from_string(owner.to_string())),
        None,
        None,
    );
    router
        .execute_contract(
            owner.clone(),
            cw20_exchange_addr.clone(),
            &redeem_msg,
            &[coin(100u128, "uusd")],
        )
        .unwrap();

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.inner().clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(cw20_exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        AssetInfo::Native("uusd".to_string())
    );
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().amount,
        Uint128::new(100)
    );
    assert_eq!(
        redeem_query_resp.redeem.unwrap().amount_paid_out,
        Uint128::zero()
    );

    // Now user1 will try to redeem 5 uandr
    let redeem_msg = mock_redeem_native_msg(Some(Recipient::from_string(user1.to_string())));

    router
        .execute_contract(
            user1.clone(),
            cw20_exchange_addr.clone(),
            &redeem_msg,
            &[coin(5u128, "uandr")],
        )
        .unwrap();

    // Check that user1 has received 10 uandr
    let balance = router.wrap().query_balance(user1.clone(), "uusd").unwrap();
    assert_eq!(balance.amount, Uint128::new(10 + 10u128));

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.inner().clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(cw20_exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        AssetInfo::Native("uusd".to_string())
    );
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().amount,
        Uint128::new(100 - 10u128)
    );

    assert_eq!(
        redeem_query_resp.redeem.unwrap().amount_paid_out,
        Uint128::new(10)
    );

    // User1 will now try to redeem 60 uandr, but he should be refunded 10 since the first 50 will deplete the redeemable amount
    let redeem_msg = mock_redeem_native_msg(Some(Recipient::from_string(user1.to_string())));

    router
        .execute_contract(
            user1.clone(),
            cw20_exchange_addr.clone(),
            &redeem_msg,
            &[coin(60u128, "uandr")],
        )
        .unwrap();

    // Check that user1 has received 5 uusd
    let balance = router.wrap().query_balance(user1.clone(), "uusd").unwrap();
    // Initial balance is 10, total redeemable is 100, 50 was redeemed, 10 was refunded
    assert_eq!(balance.amount, Uint128::new(10 + 100));

    // Query user1's uandr balance
    let balance = router.wrap().query_balance(user1.clone(), "uandr").unwrap();
    // 1000 is the original balance,
    // 5 is the amount sent in the first redeem,
    // 60 is the amount for the second redeem,
    // the last 10 is for the refund
    assert_eq!(balance.amount, Uint128::new(1000 - 60u128 + 10u128));

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.inner().clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(cw20_exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().amount,
        Uint128::zero()
    );
    assert_eq!(
        redeem_query_resp.redeem.unwrap().amount_paid_out,
        Uint128::new(100)
    );
    // User 1 will try to redeem but there is no redeemable amount left
    let redeem_msg = mock_redeem_native_msg(Some(Recipient::from_string(user1.to_string())));

    let err: ContractError = router
        .execute_contract(user1.clone(), cw20_exchange_addr.clone(), &redeem_msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::Payment(cw_utils::PaymentError::NoFunds {})
    );

    // Replenish the redeem
    let replenish_msg = mock_replenish_redeem_native_msg(uandr_asset.clone());
    router
        .execute_contract(
            owner.clone(),
            cw20_exchange_addr.clone(),
            &replenish_msg,
            &[coin(10u128, "uusd")],
        )
        .unwrap();

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.inner().clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(cw20_exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.unwrap().amount,
        Uint128::new(10u128)
    );
}

#[test]
fn test_cw20_exchange_app_redeem_native_to_cw20() {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    let addresses = get_addresses(&mut router, &andr, &app);

    let cw20_exchange_addr = addresses.cw20_exchange;
    let cw20_addr = addresses.cw20;
    let uandr_asset = AssetInfo::Native("uandr".to_string());
    // Now the owner will setup a redeem condition for 2 cw20 per cw20addr
    let start_redeem_msg = mock_start_redeem_cw20_msg(
        None,
        uandr_asset.clone(),
        Decimal256::from_ratio(Uint128::new(2), Uint128::new(1)),
        None,
        None,
    );

    let cw20_send_msg = mock_cw20_send(
        cw20_exchange_addr.clone(),
        Uint128::new(100u128),
        to_json_binary(&start_redeem_msg).unwrap(),
    );

    router
        .execute_contract(owner.clone(), cw20_addr.clone(), &cw20_send_msg, &[])
        .unwrap();

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.inner().clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(cw20_exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        AssetInfo::Cw20(cw20_addr.clone())
    );
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().amount,
        Uint128::new(100)
    );
    assert_eq!(
        redeem_query_resp.redeem.unwrap().amount_paid_out,
        Uint128::zero()
    );

    // Now user1 will try to redeem 5 uandr
    let redeem_msg = mock_redeem_native_msg(Some(Recipient::from_string(user1.to_string())));

    router
        .execute_contract(
            user1.clone(),
            cw20_exchange_addr.clone(),
            &redeem_msg,
            &[coin(5u128, "uandr")],
        )
        .unwrap();

    // Check that user1 has received 10 cw20addr
    let balance = query_cw20_balance(&mut router, cw20_addr.to_string(), user1.to_string());
    assert_eq!(balance, Uint128::new(1000 + 10));

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.inner().clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(cw20_exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        AssetInfo::Cw20(cw20_addr.clone())
    );
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().amount,
        Uint128::new(100 - 10u128)
    );

    assert_eq!(
        redeem_query_resp.redeem.unwrap().amount_paid_out,
        Uint128::new(10)
    );

    // User1 will now try to redeem 60 uandr, but he should be refunded 10 since the first 50 will deplete the redeemable amount
    let redeem_msg = mock_redeem_native_msg(Some(Recipient::from_string(user1.to_string())));

    router
        .execute_contract(
            user1.clone(),
            cw20_exchange_addr.clone(),
            &redeem_msg,
            &[coin(60u128, "uandr")],
        )
        .unwrap();

    // Check that user1 has received 5 uusd
    let balance = query_cw20_balance(&mut router, cw20_addr.to_string(), user1.to_string());
    // Initial balance is 10, total redeemable is 100, 50 was redeemed, 10 was refunded
    assert_eq!(balance, Uint128::new(1000 + 10 + 90));

    // Query user1's uandr balance
    let balance = router.wrap().query_balance(user1.clone(), "uandr").unwrap();
    // 1000 is the original balance,
    // 5 is the amount sent in the first redeem,
    // 60 is the amount for the second redeem,
    // the last 10 is for the refund
    assert_eq!(balance.amount, Uint128::new(1000 - 60u128 + 10u128));

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.inner().clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(cw20_exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().amount,
        Uint128::zero()
    );
    assert_eq!(
        redeem_query_resp.redeem.unwrap().amount_paid_out,
        Uint128::new(100)
    );
    // User 1 will try to redeem but there is no redeemable amount left
    let redeem_msg = mock_redeem_native_msg(Some(Recipient::from_string(user1.to_string())));

    let err: ContractError = router
        .execute_contract(user1.clone(), cw20_exchange_addr.clone(), &redeem_msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::Payment(cw_utils::PaymentError::NoFunds {})
    );
}

#[test]
fn test_cw20_exchange_app_redeem_native_fractional() {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    let addresses = get_addresses(&mut router, &andr, &app);

    let cw20_exchange_addr = addresses.cw20_exchange;
    let uandr_asset = AssetInfo::Native("uandr".to_string());

    let redeem_msg = mock_set_redeem_condition_native_msg(
        uandr_asset.clone(),
        Decimal256::from_ratio(Uint128::new(1), Uint128::new(2)),
        Some(Recipient::from_string(owner.to_string())),
        None,
        None,
    );
    router
        .execute_contract(
            owner.clone(),
            cw20_exchange_addr.clone(),
            &redeem_msg,
            &[coin(100u128, "uusd")],
        )
        .unwrap();

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.inner().clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(cw20_exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        AssetInfo::Native("uusd".to_string())
    );
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().amount,
        Uint128::new(100)
    );
    assert_eq!(
        redeem_query_resp.redeem.unwrap().amount_paid_out,
        Uint128::zero()
    );

    // Now user1 will try to redeem 10 uandr
    let redeem_msg = mock_redeem_native_msg(Some(Recipient::from_string(user1.to_string())));

    router
        .execute_contract(
            user1.clone(),
            cw20_exchange_addr.clone(),
            &redeem_msg,
            &[coin(10u128, "uandr")],
        )
        .unwrap();

    // Check that user1 has received 5 uusd
    let balance = router.wrap().query_balance(user1.clone(), "uusd").unwrap();
    assert_eq!(balance.amount, Uint128::new(10 + 5u128));

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.inner().clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(cw20_exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        AssetInfo::Native("uusd".to_string())
    );
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().amount,
        Uint128::new(100 - 5u128)
    );

    assert_eq!(
        redeem_query_resp.redeem.unwrap().amount_paid_out,
        Uint128::new(5)
    );

    // User1 will now try to redeem 60 cw20addr2, but he should be refunded 10 since the first 50 will deplete the redeemable amount
    let redeem_msg = mock_redeem_native_msg(Some(Recipient::from_string(user1.to_string())));

    router
        .execute_contract(
            user1.clone(),
            cw20_exchange_addr.clone(),
            &redeem_msg,
            &[coin(200u128, "uandr")],
        )
        .unwrap();

    // Check that user1 has received 5 uusd
    let balance = router.wrap().query_balance(user1.clone(), "uusd").unwrap();
    // Initial balance is 10, total redeemable is 100, 50 was redeemed, 10 was refunded
    assert_eq!(balance.amount, Uint128::new(10 + 100));

    // Query user1's uandr balance
    let balance = router.wrap().query_balance(user1.clone(), "uandr").unwrap();
    // 1000 is the original balance,
    // 10 is the amount sent in the first redeem,
    // 200 is the amount for the second redeem,
    // the last 10 is for the refund
    assert_eq!(balance.amount, Uint128::new(1000 - 10 - 200u128 + 10u128));

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.inner().clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(cw20_exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().amount,
        Uint128::zero()
    );
    assert_eq!(
        redeem_query_resp.redeem.unwrap().amount_paid_out,
        Uint128::new(100)
    );
    // User 1 will try to redeem but there is no redeemable amount left
    let redeem_msg = mock_redeem_native_msg(Some(Recipient::from_string(user1.to_string())));

    let err: ContractError = router
        .execute_contract(user1.clone(), cw20_exchange_addr.clone(), &redeem_msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::Payment(cw_utils::PaymentError::NoFunds {})
    );
}
