use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, mock_app_instantiate_msg, MockAppContract};
use andromeda_cw20::mock::{
    mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_cw20_send, mock_minter,
};
use andromeda_osmosis_token_factory::mock::{
    mock_andromeda_osmosis_token_factory, mock_create_denom, mock_cw20_hook_msg,
    mock_osmosis_token_factory_instantiate_msg, query_all_locked,
};
use andromeda_socket::osmosis_token_factory::AllLockedResponse;
use andromeda_testing::{
    mock::{mock_app, MockAndromeda, MockApp},
    mock_builder::MockAndromedaBuilder,
    MockContract,
};
use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Timestamp, Uint128};
use cw20::Cw20Coin;

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
            (
                "osmosis-token-factory",
                mock_andromeda_osmosis_token_factory(),
            ),
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

    let osmosis_token_factory_init_msg = mock_osmosis_token_factory_instantiate_msg(
        andr.kernel.addr().to_string(),
        Some(owner.to_string()),
    );
    let osmosis_token_factory_component = AppComponent::new(
        "osmosis-token-factory".to_string(),
        "osmosis-token-factory".to_string(),
        to_json_binary(&osmosis_token_factory_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![
        cw20_component_1,
        cw20_component_2,
        osmosis_token_factory_component,
    ];
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

fn _advance_time(router: &mut MockApp, seconds: u64) {
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: Timestamp::from_seconds(router.block_info().time.seconds() + seconds),
        chain_id: router.block_info().chain_id,
    });
}

struct TestAddresses {
    cw20_2: Addr,
    osmosis_token_factory: Addr,
}

fn get_addresses(
    router: &mut MockApp,
    andr: &MockAndromeda,
    app: &MockAppContract,
) -> TestAddresses {
    TestAddresses {
        cw20_2: andr
            .vfs
            .query_resolve_path(router, format!("/home/{}/cw20-2", app.addr())),
        osmosis_token_factory: andr.vfs.query_resolve_path(
            router,
            format!("/home/{}/osmosis-token-factory", app.addr()),
        ),
    }
}

const ORIGINAL_SALE_AMOUNT: Uint128 = Uint128::new(1000u128);

// This test was used to debug cw20receive for the osmosis token factory
#[test]
fn test_exchange_app_cw20_to_cw20() {
    let mut router = mock_app(None);

    let andr = setup_andr(&mut router);
    let app = setup_app(&andr, &mut router);
    let owner = andr.get_wallet("owner");

    let addresses = get_addresses(&mut router, &andr, &app);

    let _cw20_addr_2 = addresses.cw20_2;
    let osmosis_token_factory_addr = addresses.osmosis_token_factory;

    // Make sure contract exists
    let query_msg: AllLockedResponse = router
        .wrap()
        .query_wasm_smart(osmosis_token_factory_addr.clone(), &query_all_locked())
        .unwrap();
    println!("query_msg {:?}", query_msg);

    // Create a denom
    let create_denom_msg = mock_create_denom("test".to_string());
    router
        .execute_contract(
            owner.clone(),
            osmosis_token_factory_addr.clone(),
            &create_denom_msg,
            &[],
        )
        .unwrap();

    let _query_msg: AllLockedResponse = router
        .wrap()
        .query_wasm_smart(osmosis_token_factory_addr.clone(), &query_all_locked())
        .unwrap();
    // Sell a cw20
    let lock_msg = mock_cw20_hook_msg(None);

    let _cw20_send_msg = mock_cw20_send(
        osmosis_token_factory_addr.clone(),
        ORIGINAL_SALE_AMOUNT,
        to_json_binary(&lock_msg).unwrap(),
    );

    // router
    //     .execute_contract(owner.clone(), cw20_addr_2.clone(), &cw20_send_msg, &[])
    //     .unwrap();
}
