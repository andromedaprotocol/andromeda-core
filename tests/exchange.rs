use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, mock_app_instantiate_msg, MockAppContract};
use andromeda_cw20::mock::{
    mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_cw20_send, mock_get_cw20_balance,
    mock_minter,
};
use andromeda_exchange::mock::{
    mock_andromeda_exchange, mock_exchange_hook_purchase_msg, mock_exchange_instantiate_msg,
    mock_exchange_start_sale_msg, mock_redeem_cw20_msg, mock_redeem_native_msg,
    mock_redeem_query_msg, mock_replenish_redeem_cw20_msg, mock_replenish_redeem_native_msg,
    mock_set_redeem_condition_native_msg, mock_start_redeem_cw20_msg, MockExchange,
};
use andromeda_fungible_tokens::exchange::{ExchangeRate, RedeemResponse, SaleResponse};
use andromeda_std::{
    amp::{AndrAddr, Recipient},
    common::{denom::Asset, schedule::Schedule},
    error::ContractError,
};
use andromeda_testing::{
    mock::{mock_app, MockAndromeda, MockApp},
    mock_builder::MockAndromedaBuilder,
    MockContract,
};
use cosmwasm_std::{
    coin, to_json_binary, Addr, BlockInfo, Decimal256, Timestamp, Uint128, Uint256,
};
use cw20::{BalanceResponse, Cw20Coin};
use rstest::rstest;

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
            ("exchange", mock_andromeda_exchange()),
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

    let exchange_init_msg = mock_exchange_instantiate_msg(
        AndrAddr::from_string("./cw20-2"),
        andr.kernel.addr().to_string(),
        Some(owner.to_string()),
    );
    let exchange_component = AppComponent::new(
        "exchange".to_string(),
        "exchange".to_string(),
        to_json_binary(&exchange_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![cw20_component_1, cw20_component_2, exchange_component];
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
    exchange: Addr,
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
        exchange: andr
            .vfs
            .query_resolve_path(router, format!("/home/{}/exchange", app.addr())),
    }
}

const ORIGINAL_SALE_AMOUNT: Uint128 = Uint128::new(1000u128);

#[test]
fn test_exchange_app_cw20_to_native() {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    let addresses = get_addresses(&mut router, &andr, &app);

    let cw20_addr = addresses.cw20;
    let cw20_addr_2 = addresses.cw20_2;
    let exchange_addr = addresses.exchange;

    let cw20_addr_2_asset = Asset::Cw20Token(AndrAddr::from_string(cw20_addr_2.to_string()));
    let cw20_redeem_asset = Asset::Cw20Token(AndrAddr::from_string(cw20_addr.to_string()));

    let exchange: MockExchange = app.query_ado_by_component_name(&router, "exchange");

    // Sell a cw20
    exchange.execute_cw20_start_sale(
        &mut router,
        owner.clone(),
        cw20_redeem_asset.clone(),
        ORIGINAL_SALE_AMOUNT,
        Uint128::new(2),
        cw20_addr_2.clone(),
        Schedule::default(),
    );

    // Now there's a sale for cw20addr2 for 2 cw20addr per token
    // user1 will purchase 10 cw20addr2
    exchange.execute_cw20_purchase(
        &mut router,
        user1.clone(),
        Some(Recipient::from_string(user1.to_string())),
        Uint128::new(10u128),
        cw20_addr.clone(),
    );

    // Check that user1 has received 5 cw20addr_2
    let balance = query_cw20_balance(&mut router, cw20_addr_2.to_string(), user1.to_string());
    assert_eq!(balance, Uint128::new(1000 + 5u128));

    // Now the owner will setup a redeem condition for 2 uandr per cw20addr
    let redeem_msg = mock_set_redeem_condition_native_msg(
        cw20_addr_2_asset.clone(),
        ExchangeRate::Fixed(Decimal256::from_ratio(Uint128::new(2), Uint128::new(1))),
        Some(Recipient::from_string(owner.to_string())),
        Schedule::default(),
    );
    router
        .execute_contract(
            owner.clone(),
            exchange_addr.clone(),
            &redeem_msg,
            &[coin(100u128, "uandr")],
        )
        .unwrap();

    let redeem_query_resp: RedeemResponse =
        exchange.query_redeem(&mut router, cw20_addr_2_asset.clone());
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        Asset::NativeToken("uandr".to_string())
    );
    assert_eq!(redeem_query_resp.redeem.unwrap().amount, Uint128::new(100));

    // Now user1 will try to redeem 5 cw20addr_2
    let redeem_msg = mock_redeem_cw20_msg(Some(Recipient::from_string(user1.to_string())));
    let cw20_send_msg = mock_cw20_send(
        exchange_addr.clone(),
        Uint128::new(5u128),
        to_json_binary(&redeem_msg).unwrap(),
    );

    router
        .execute_contract(user1.clone(), cw20_addr_2.clone(), &cw20_send_msg, &[])
        .unwrap();

    // Check that user1 has received 10 uandr
    let balance = router.wrap().query_balance(user1.clone(), "uandr").unwrap();
    assert_eq!(balance.amount, Uint128::new(1000 + 10u128));

    let redeem_query_msg = mock_redeem_query_msg(cw20_addr_2_asset.clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        Asset::NativeToken("uandr".to_string())
    );
    assert_eq!(
        redeem_query_resp.redeem.unwrap().amount,
        Uint128::new(100 - 10u128)
    );

    // User1 will now try to redeem 60 cw20addr2, but he should be refunded 10 since the first 50 will deplete the redeemable amount
    let redeem_msg = mock_redeem_cw20_msg(Some(Recipient::from_string(user1.to_string())));
    let cw20_send_msg = mock_cw20_send(
        exchange_addr.clone(),
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

    let redeem_query_msg = mock_redeem_query_msg(cw20_addr_2_asset.clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(redeem_query_resp.redeem.unwrap().amount, Uint128::zero());

    // User 1 will try to redeem but there is no redeemable amount left
    let redeem_msg = mock_redeem_cw20_msg(Some(Recipient::from_string(user1.to_string())));
    let cw20_send_msg = mock_cw20_send(
        exchange_addr.clone(),
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
fn test_exchange_app_cw20_to_cw20() {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    let addresses = get_addresses(&mut router, &andr, &app);

    let cw20_addr = addresses.cw20;
    let cw20_addr_2 = addresses.cw20_2;
    let exchange_addr = addresses.exchange;

    let cw20_addr_2_asset = Asset::Cw20Token(AndrAddr::from_string(cw20_addr_2.to_string()));
    let cw20_redeem_asset = Asset::Cw20Token(AndrAddr::from_string(cw20_addr.to_string()));

    // Sell a cw20
    let start_sale_msg = mock_exchange_start_sale_msg(
        cw20_redeem_asset,
        Uint128::new(2),
        None,
        Schedule::default(),
    );

    let cw20_send_msg = mock_cw20_send(
        exchange_addr.clone(),
        ORIGINAL_SALE_AMOUNT,
        to_json_binary(&start_sale_msg).unwrap(),
    );

    router
        .execute_contract(owner.clone(), cw20_addr_2.clone(), &cw20_send_msg, &[])
        .unwrap();

    // Now there's a sale for cw20addr2 for 2 cw20addr per token
    // user1 will purchase 10 cw20addr2

    let purchase_msg =
        mock_exchange_hook_purchase_msg(Some(Recipient::from_string(user1.to_string())));
    let cw20_send_msg = mock_cw20_send(
        exchange_addr.clone(),
        Uint128::new(10u128),
        to_json_binary(&purchase_msg).unwrap(),
    );

    router
        .execute_contract(user1.clone(), cw20_addr.clone(), &cw20_send_msg, &[])
        .unwrap();

    // Check that user1 has received 5 cw20addr_2
    let balance = query_cw20_balance(&mut router, cw20_addr_2.to_string(), user1.to_string());
    assert_eq!(balance, Uint128::new(1000 + 5u128));

    let exchange_rate =
        ExchangeRate::Fixed(Decimal256::from_ratio(Uint128::new(2), Uint128::new(1)));
    // Now the owner will setup a redeem condition for 2 cw20 per cw20addr
    let start_redeem_msg = mock_start_redeem_cw20_msg(
        None,
        cw20_addr_2_asset.clone(),
        exchange_rate.clone(),
        Schedule::default(),
    );

    let cw20_send_msg = mock_cw20_send(
        exchange_addr.clone(),
        Uint128::new(100u128),
        to_json_binary(&start_redeem_msg).unwrap(),
    );

    router
        .execute_contract(owner.clone(), cw20_addr.clone(), &cw20_send_msg, &[])
        .unwrap();

    let redeem_query_msg = mock_redeem_query_msg(cw20_addr_2_asset.clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        Asset::Cw20Token(AndrAddr::from_string(cw20_addr.to_string()))
    );
    assert_eq!(redeem_query_resp.redeem.unwrap().amount, Uint128::new(100));

    // Now user1 will try to redeem 5 cw20addr_2
    let redeem_msg = mock_redeem_cw20_msg(Some(Recipient::from_string(user1.to_string())));
    let cw20_send_msg = mock_cw20_send(
        exchange_addr.clone(),
        Uint128::new(5u128),
        to_json_binary(&redeem_msg).unwrap(),
    );

    router
        .execute_contract(user1.clone(), cw20_addr_2.clone(), &cw20_send_msg, &[])
        .unwrap();

    // 1000 is the original balance, 10 is the amount purchased with previously in the test, 10 is the amount received in the redeem
    let balance = query_cw20_balance(&mut router, cw20_addr.to_string(), user1.to_string());
    assert_eq!(balance, Uint128::new(1000 - 10 + 10u128));

    let redeem_query_msg = mock_redeem_query_msg(cw20_addr_2_asset.clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        Asset::Cw20Token(AndrAddr::from_string(cw20_addr.to_string()))
    );
    assert_eq!(
        redeem_query_resp.redeem.unwrap().amount,
        Uint128::new(100 - 10u128)
    );

    // User1 will now try to redeem 60 cw20addr2, but he should be refunded 10 since the first 50 will deplete the redeemable amount
    let redeem_msg = mock_redeem_cw20_msg(Some(Recipient::from_string(user1.to_string())));
    let cw20_send_msg = mock_cw20_send(
        exchange_addr.clone(),
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

    let redeem_query_msg = mock_redeem_query_msg(cw20_addr_2_asset.clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(redeem_query_resp.redeem.unwrap().amount, Uint128::zero());

    // User 1 will try to redeem but there is no redeemable amount left
    let redeem_msg = mock_redeem_cw20_msg(Some(Recipient::from_string(user1.to_string())));
    let cw20_send_msg = mock_cw20_send(
        exchange_addr.clone(),
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
    let replenish_msg = mock_replenish_redeem_cw20_msg(cw20_addr_2_asset.clone(), None);
    let cw20_send_msg = mock_cw20_send(
        exchange_addr.clone(),
        Uint128::new(10u128),
        to_json_binary(&replenish_msg).unwrap(),
    );

    router
        .execute_contract(owner.clone(), cw20_addr.clone(), &cw20_send_msg, &[])
        .unwrap();

    let redeem_query_msg = mock_redeem_query_msg(cw20_addr_2_asset.clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.unwrap().amount,
        Uint128::new(10u128)
    );
}

#[test]
fn test_exchange_app_cancel_sale() {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");

    let addresses = get_addresses(&mut router, &andr, &app);

    let cw20_addr = addresses.cw20;
    let cw20_addr_2 = addresses.cw20_2;

    let cw20_redeem_asset = Asset::Cw20Token(AndrAddr::from_string(cw20_addr.to_string()));

    let exchange: MockExchange = app.query_ado_by_component_name(&router, "exchange");

    // Sell a cw20
    exchange.execute_cw20_start_sale(
        &mut router,
        owner.clone(),
        cw20_redeem_asset.clone(),
        ORIGINAL_SALE_AMOUNT,
        Uint128::new(2),
        cw20_addr_2.clone(),
        Schedule::default(),
    );

    // Query to see that the sale exists
    let sale_query_resp: SaleResponse = exchange.query_sale(&mut router, cw20_addr.to_string());
    assert!(sale_query_resp.sale.is_some());

    // Cancel the sale
    exchange.execute_cancel_sale(&mut router, owner.clone(), cw20_redeem_asset.clone());

    // Query to see that the sale does not exist
    let sale_query_resp: SaleResponse = exchange.query_sale(&mut router, cw20_addr.to_string());
    assert!(sale_query_resp.sale.is_none());
}

#[test]
fn test_exchange_app_cancel_redeem() {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");

    let addresses = get_addresses(&mut router, &andr, &app);

    let cw20_addr = addresses.cw20;
    let cw20_addr_2 = addresses.cw20_2;

    let cw20_addr_2_asset = Asset::Cw20Token(AndrAddr::from_string(cw20_addr_2.to_string()));

    //TODO: Rename to exchange, when the other PRs are merged
    let exchange: MockExchange = app.query_ado_by_component_name(&router, "exchange");

    // Now the owner will setup a redeem condition for 2 cw20 per cw20addr
    let exchange_rate =
        ExchangeRate::Fixed(Decimal256::from_ratio(Uint128::new(2), Uint128::new(1)));
    exchange.execute_cw20_start_redeem(
        &mut router,
        owner.clone(),
        cw20_addr_2_asset.clone(),
        Uint128::new(100),
        exchange_rate,
        cw20_addr.clone(),
        Schedule::default(),
    );

    // Query to see that the redeem exists
    let redeem_query_resp: RedeemResponse =
        exchange.query_redeem(&mut router, cw20_addr_2_asset.clone());
    assert!(redeem_query_resp.redeem.is_some());

    // Cancel the redeem
    exchange.execute_cancel_redeem(&mut router, owner.clone(), cw20_addr_2_asset.clone());

    // Query to see that the redeem does not exist
    let redeem_query_resp: RedeemResponse =
        exchange.query_redeem(&mut router, cw20_addr_2_asset.clone());
    assert!(redeem_query_resp.redeem.is_none());
}

#[test]
fn test_exchange_app_redeem_native_to_native() {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    let addresses = get_addresses(&mut router, &andr, &app);

    let exchange_addr = addresses.exchange;
    let uandr_asset = Asset::NativeToken("uandr".to_string());

    let exchange_rate =
        ExchangeRate::Fixed(Decimal256::from_ratio(Uint128::new(2), Uint128::new(1)));
    // Now the owner will setup a redeem condition for 2 uandr per uusd
    let redeem_msg = mock_set_redeem_condition_native_msg(
        uandr_asset.clone(),
        exchange_rate.clone(),
        Some(Recipient::from_string(owner.to_string())),
        Schedule::default(),
    );
    router
        .execute_contract(
            owner.clone(),
            exchange_addr.clone(),
            &redeem_msg,
            &[coin(100u128, "uusd")],
        )
        .unwrap();

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        Asset::NativeToken("uusd".to_string())
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
            exchange_addr.clone(),
            &redeem_msg,
            &[coin(5u128, "uandr")],
        )
        .unwrap();

    // Check that user1 has received 10 uusd
    let balance = router.wrap().query_balance(user1.clone(), "uusd").unwrap();
    assert_eq!(
        balance.amount,
        Uint128::new(USER1_INITIAL_BALANCE.u128() + 10u128)
    );

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        Asset::NativeToken("uusd".to_string())
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
            exchange_addr.clone(),
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

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(exchange_addr.clone(), &redeem_query_msg)
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
        .execute_contract(user1.clone(), exchange_addr.clone(), &redeem_msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::Payment(cw_utils::PaymentError::NoFunds {})
    );

    // Replenish the redeem
    let replenish_msg = mock_replenish_redeem_native_msg(uandr_asset.clone(), Some(exchange_rate));
    router
        .execute_contract(
            owner.clone(),
            exchange_addr.clone(),
            &replenish_msg,
            &[coin(10u128, "uusd")],
        )
        .unwrap();

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.unwrap().amount,
        Uint128::new(10u128)
    );
}

#[test]
fn test_exchange_app_redeem_native_to_cw20() {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    let addresses = get_addresses(&mut router, &andr, &app);

    let exchange_addr = addresses.exchange;
    let cw20_addr = addresses.cw20;
    let uandr_asset = Asset::NativeToken("uandr".to_string());
    // Now the owner will setup a redeem condition for 2 cw20 per cw20addr
    let start_redeem_msg = mock_start_redeem_cw20_msg(
        None,
        uandr_asset.clone(),
        ExchangeRate::Fixed(Decimal256::from_ratio(Uint128::new(2), Uint128::new(1))),
        Schedule::default(),
    );

    let cw20_send_msg = mock_cw20_send(
        exchange_addr.clone(),
        Uint128::new(100u128),
        to_json_binary(&start_redeem_msg).unwrap(),
    );

    router
        .execute_contract(owner.clone(), cw20_addr.clone(), &cw20_send_msg, &[])
        .unwrap();

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        Asset::Cw20Token(AndrAddr::from_string(cw20_addr.to_string()))
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
            exchange_addr.clone(),
            &redeem_msg,
            &[coin(5u128, "uandr")],
        )
        .unwrap();

    // Check that user1 has received 10 cw20addr
    let balance = query_cw20_balance(&mut router, cw20_addr.to_string(), user1.to_string());
    assert_eq!(balance, Uint128::new(1000 + 10));

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        Asset::Cw20Token(AndrAddr::from_string(cw20_addr.to_string()))
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
            exchange_addr.clone(),
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

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(exchange_addr.clone(), &redeem_query_msg)
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
        .execute_contract(user1.clone(), exchange_addr.clone(), &redeem_msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::Payment(cw_utils::PaymentError::NoFunds {})
    );
}

#[test]
fn test_exchange_app_redeem_native_fractional() {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    let addresses = get_addresses(&mut router, &andr, &app);

    let exchange_addr = addresses.exchange;
    let uandr_asset = Asset::NativeToken("uandr".to_string());

    let redeem_msg = mock_set_redeem_condition_native_msg(
        uandr_asset.clone(),
        ExchangeRate::Fixed(Decimal256::from_ratio(Uint128::new(1), Uint128::new(2))),
        Some(Recipient::from_string(owner.to_string())),
        Schedule::default(),
    );
    router
        .execute_contract(
            owner.clone(),
            exchange_addr.clone(),
            &redeem_msg,
            &[coin(100u128, "uusd")],
        )
        .unwrap();

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        Asset::NativeToken("uusd".to_string())
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
            exchange_addr.clone(),
            &redeem_msg,
            &[coin(10u128, "uandr")],
        )
        .unwrap();

    // Check that user1 has received 5 uusd
    let balance = router.wrap().query_balance(user1.clone(), "uusd").unwrap();
    assert_eq!(
        balance.amount,
        Uint128::new(USER1_INITIAL_BALANCE.u128() + 5u128)
    );

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        Asset::NativeToken("uusd".to_string())
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
            exchange_addr.clone(),
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

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(exchange_addr.clone(), &redeem_query_msg)
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
        .execute_contract(user1.clone(), exchange_addr.clone(), &redeem_msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::Payment(cw_utils::PaymentError::NoFunds {})
    );
}

#[rstest]
#[case::variable_rate_200(
    Uint256::from(200u128),
    100u128,
    Decimal256::from_ratio(Uint128::new(1), Uint128::new(2))
)]
#[case::variable_rate_100(Uint256::from(100u128), 100u128, Decimal256::one())]
#[case::variable_rate_50(
    Uint256::from(50u128),
    100u128,
    Decimal256::from_ratio(Uint128::new(2), Uint128::new(1))
)]

fn test_exchange_app_redeem_native_to_native_dynamic_exchange_rate(
    #[case] variable_rate: Uint256,
    #[case] amount_sent: u128,
    #[case] expected_exchange_rate: Decimal256,
) {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");

    let addresses = get_addresses(&mut router, &andr, &app);

    let exchange_addr = addresses.exchange;
    let uandr_asset = Asset::NativeToken("uandr".to_string());

    // Setup redeem condition with variable exchange rate
    let redeem_msg = mock_set_redeem_condition_native_msg(
        uandr_asset.clone(),
        ExchangeRate::Variable(variable_rate),
        Some(Recipient::from_string(owner.to_string())),
        Schedule::default(),
    );
    router
        .execute_contract(
            owner.clone(),
            exchange_addr.clone(),
            &redeem_msg,
            &[coin(amount_sent, "uusd")],
        )
        .unwrap();

    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().asset,
        Asset::NativeToken("uusd".to_string())
    );
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().amount,
        Uint128::new(amount_sent)
    );
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().amount_paid_out,
        Uint128::zero()
    );
    assert_eq!(
        redeem_query_resp.redeem.clone().unwrap().exchange_rate,
        expected_exchange_rate
    );
}

#[rstest]
#[case::none_exchange_rate(
    None, // replenish_exchange_rate
    1000u128, // replenish_amount
    Decimal256::percent(100), // expected_decimal
    "None exchange rate" // description
)]
#[case::fixed_exchange_rate(
    Some(ExchangeRate::Fixed(Decimal256::percent(150))),
    1000u128,
    Decimal256::percent(150), // Same as the replenish exchange rate
    "Fixed exchange rate"
)]
#[case::variable_exchange_rate(
    Some(ExchangeRate::Variable(Uint256::from(2000u128))),
    1000u128,
    Decimal256::from_ratio(Uint128::new(1), Uint128::new(1)), // New exchange will be 1000 + 1000 / 2000 = 1
    "Variable exchange rate should change"
)]
fn test_replenish_redeem_native_with_various_exchange_rates(
    #[case] replenish_exchange_rate: Option<ExchangeRate>,
    #[case] replenish_amount: u128,
    #[case] expected_decimal: Decimal256,
    #[case] description: &str,
) {
    let mut router = mock_app(None);
    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");
    let addresses = get_addresses(&mut router, &andr, &app);
    let exchange_addr = addresses.exchange;
    let uandr_asset = Asset::NativeToken("uandr".to_string());

    // Initial setup for redeem condition with Fixed(100%)
    let initial_amount_sent = 1000u128;
    let initial_exchange_rate = ExchangeRate::Fixed(Decimal256::percent(100));
    let redeem_msg = mock_set_redeem_condition_native_msg(
        uandr_asset.clone(),
        initial_exchange_rate.clone(),
        Some(Recipient::from_string(owner.to_string())),
        Schedule::default(),
    );
    router
        .execute_contract(
            owner.clone(),
            exchange_addr.clone(),
            &redeem_msg,
            &[coin(initial_amount_sent, "uusd")],
        )
        .unwrap();

    // Replenish with the parameterized exchange rate
    let replenish_msg =
        mock_replenish_redeem_native_msg(uandr_asset.clone(), replenish_exchange_rate.clone());
    router
        .execute_contract(
            owner.clone(),
            exchange_addr.clone(),
            &replenish_msg,
            &[coin(replenish_amount, "uusd")],
        )
        .unwrap();

    // Query to verify replenish and exchange rate
    let redeem_query_msg = mock_redeem_query_msg(uandr_asset.clone());
    let redeem_query_resp: RedeemResponse = router
        .wrap()
        .query_wasm_smart(exchange_addr.clone(), &redeem_query_msg)
        .unwrap();
    let new_redeem = redeem_query_resp.redeem.unwrap();
    assert_eq!(
        new_redeem.amount,
        Uint128::new(initial_amount_sent + replenish_amount),
        "Total amount should be the sum of the initial amount and the replenish amount {} {}",
        initial_amount_sent,
        replenish_amount,
    );
    assert_eq!(
        new_redeem.exchange_rate, expected_decimal,
        "{}",
        description
    );
}
