use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{
    mock_andromeda_app, mock_app_instantiate_msg, mock_get_components_msg,
};
use andromeda_cw20::mock::{
    mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_cw20_send, mock_cw20_transfer,
    mock_get_cw20_balance, mock_get_version, mock_minter,
};
use andromeda_cw20_staking::mock::{
    mock_andromeda_cw20_staking, mock_cw20_get_staker, mock_cw20_stake,
    mock_cw20_staking_add_reward_tokens, mock_cw20_staking_instantiate_msg,
    mock_cw20_staking_update_global_indexes,
};
use andromeda_fungible_tokens::cw20_staking::{AllocationConfig, StakerResponse};

use andromeda_std::common::Milliseconds;

use andromeda_std::ado_base::version::VersionResponse;
use andromeda_testing::mock::{mock_app, MockAndromeda, MockApp};
use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Timestamp, Uint128};
use cw20::{BalanceResponse, Cw20Coin};
use cw_asset::AssetInfoUnchecked;
use cw_multi_test::Executor;

fn mock_andromeda(app: &mut MockApp, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

fn setup_andr(router: &mut MockApp, owner: &Addr) -> MockAndromeda {
    let andr = mock_andromeda(router, owner.clone());

    // Store contract codes
    let cw20_code_id = router.store_code(mock_andromeda_cw20());
    let cw20_staking_code_id = router.store_code(mock_andromeda_cw20_staking());
    let app_code_id = router.store_code(mock_andromeda_app());
    andr.store_code_id(router, "cw20", cw20_code_id);
    andr.store_code_id(router, "cw20-staking", cw20_staking_code_id);
    andr.store_code_id(router, "app-contract", app_code_id);

    andr
}

fn setup_app(andr: &MockAndromeda, router: &mut MockApp) -> Addr {
    let owner = router.api().addr_make("owner");
    let staker_one = router.api().addr_make("staker_one");
    let staker_two = router.api().addr_make("staker_two");

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
        None,
        "Test Tokens".to_string(),
        "TTT".to_string(),
        6,
        initial_balances,
        Some(mock_minter(
            owner.to_string(),
            Some(Uint128::from(1000000u128)),
        )),
        None,
        andr.kernel_address.to_string(),
    );
    let cw20_component = AppComponent::new(
        "cw20".to_string(),
        "cw20".to_string(),
        to_json_binary(&cw20_init_msg).unwrap(),
    );

    let cw20_staking_init_msg = mock_cw20_staking_instantiate_msg(
        format!("./{}", cw20_component.name),
        andr.kernel_address.to_string(),
        None,
        None,
    );
    let cw20_staking_component = AppComponent::new(
        "cw20staking".to_string(),
        "cw20-staking".to_string(),
        to_json_binary(&cw20_staking_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![cw20_component, cw20_staking_component];
    let app_init_msg = mock_app_instantiate_msg(
        "Staking App".to_string(),
        app_components.clone(),
        andr.kernel_address.clone(),
        None,
    );

    let app_code_id = andr.get_code_id(router, "app-contract");
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

    app_addr
}

#[test]
fn test_cw20_staking_app() {
    let mut router = mock_app();
    let owner = router.api().addr_make("owner");
    let staker_one = router.api().addr_make("staker_one");
    let staker_two = router.api().addr_make("staker_two");

    let andr = setup_andr(&mut router, &owner);
    let app_addr = setup_app(&andr, &mut router);

    // Component Addresses
    let cw20_addr = andr.vfs_resolve_path(&mut router, format!("/home/{app_addr}/cw20"));
    let cw20_staking_addr =
        andr.vfs_resolve_path(&mut router, format!("/home/{app_addr}/cw20staking"));

    // Check Balances
    let balance_one: BalanceResponse = router
        .wrap()
        .query_wasm_smart(
            cw20_addr.clone(),
            &mock_get_cw20_balance(staker_one.to_string()),
        )
        .unwrap();

    let version: VersionResponse = router
        .wrap()
        .query_wasm_smart(cw20_addr.clone(), &mock_get_version())
        .unwrap();
    assert_eq!(version.version, "1.0.0-rc.1");

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
        cw20_staking_addr.to_string(),
        Uint128::from(1000u128),
        to_json_binary(&mock_cw20_stake()).unwrap(),
    );
    router
        .execute_contract(staker_one.clone(), cw20_addr.clone(), &staking_msg_one, &[])
        .unwrap();

    let staking_msg_two = mock_cw20_send(
        cw20_staking_addr.to_string(),
        Uint128::from(2000u128),
        to_json_binary(&mock_cw20_stake()).unwrap(),
    );
    router
        .execute_contract(staker_two.clone(), cw20_addr.clone(), &staking_msg_two, &[])
        .unwrap();

    // Transfer Tokens for Reward
    let transfer_msg = mock_cw20_transfer(cw20_staking_addr.to_string(), Uint128::from(3000u128));
    router
        .execute_contract(owner, cw20_addr, &transfer_msg, &[])
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

#[test]
fn test_cw20_staking_app_delayed() {
    let mut router = mock_app();
    let owner = router.api().addr_make("owner");
    let staker_one = router.api().addr_make("staker_one");
    let staker_two = router.api().addr_make("staker_two");
    router
        .send_tokens(
            Addr::unchecked("owner"),
            owner.clone(),
            &[coin(1000u128, "uandr"), coin(1000u128, "uusd")],
        )
        .unwrap();

    let andr = setup_andr(&mut router, &owner);
    let app_addr = setup_app(&andr, &mut router);

    // Component Addresses
    let cw20_addr = andr.vfs_resolve_path(&mut router, format!("/home/{app_addr}/cw20"));
    let cw20_staking_addr =
        andr.vfs_resolve_path(&mut router, format!("/home/{app_addr}/cw20staking"));

    let reward_token = AssetInfoUnchecked::native("uandr");
    let add_reward_msg = mock_cw20_staking_add_reward_tokens(
        reward_token,
        Milliseconds::from_seconds(router.block_info().time.seconds() + 1),
        None,
    );
    router
        .execute_contract(
            owner.clone(),
            cw20_staking_addr.clone(),
            &add_reward_msg,
            &[],
        )
        .unwrap();

    let reward_token_two = AssetInfoUnchecked::native("uusd");
    let add_reward_msg = mock_cw20_staking_add_reward_tokens(
        reward_token_two,
        Milliseconds::from_seconds(router.block_info().time.seconds() + 1),
        Some(AllocationConfig {
            till_timestamp: Milliseconds::from_seconds(router.block_info().time.seconds() + 101),
            cycle_rewards: Uint128::from(3u128),
            cycle_duration: Milliseconds::from_seconds(1),
            reward_increase: None,
        }),
    );
    router
        .execute_contract(
            owner.clone(),
            cw20_staking_addr.clone(),
            &add_reward_msg,
            &[],
        )
        .unwrap();

    router
        .send_tokens(
            owner.clone(),
            cw20_staking_addr.clone(),
            &[coin(60u128, "uandr")],
        )
        .unwrap();
    router
        .send_tokens(
            owner.clone(),
            cw20_staking_addr.clone(),
            &[coin(300u128, "uusd")],
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
        cw20_staking_addr.to_string(),
        Uint128::from(1000u128),
        to_json_binary(&mock_cw20_stake()).unwrap(),
    );
    router
        .execute_contract(staker_one.clone(), cw20_addr.clone(), &staking_msg_one, &[])
        .unwrap();

    let staking_msg_two = mock_cw20_send(
        cw20_staking_addr.to_string(),
        Uint128::from(2000u128),
        to_json_binary(&mock_cw20_stake()).unwrap(),
    );
    router
        .execute_contract(staker_two.clone(), cw20_addr.clone(), &staking_msg_two, &[])
        .unwrap();

    // Transfer Tokens for Reward
    let transfer_msg = mock_cw20_transfer(cw20_staking_addr.to_string(), Uint128::from(3000u128));
    router
        .execute_contract(owner.clone(), cw20_addr, &transfer_msg, &[])
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
    assert_eq!(staker_one_info.pending_rewards.len(), 2);
    for (_, reward) in staker_one_info.pending_rewards {
        assert_eq!(reward, Uint128::zero());
    }

    let staker_two_info: StakerResponse = router
        .wrap()
        .query_wasm_smart(
            cw20_staking_addr.clone(),
            &mock_cw20_get_staker(staker_two.to_string()),
        )
        .unwrap();
    assert_eq!(staker_two_info.share, Uint128::from(2000u128));
    assert_eq!(staker_two_info.balance, Uint128::from(4000u128));
    assert_eq!(staker_two_info.pending_rewards.len(), 2);
    for (_, reward) in staker_two_info.pending_rewards {
        assert_eq!(reward, Uint128::zero());
    }

    // Advance Time
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: Timestamp::from_seconds(router.block_info().time.seconds() + 51),
        chain_id: router.block_info().chain_id,
    });

    let update_global_indexes =
        mock_cw20_staking_update_global_indexes(Some(vec![AssetInfoUnchecked::native("uandr")]));
    router
        .execute_contract(
            owner,
            cw20_staking_addr.clone(),
            &update_global_indexes,
            &[],
        )
        .unwrap();

    let staker_one_info: StakerResponse = router
        .wrap()
        .query_wasm_smart(
            cw20_staking_addr.clone(),
            &mock_cw20_get_staker(staker_one.to_string()),
        )
        .unwrap();
    assert_eq!(staker_one_info.share, Uint128::from(1000u128));
    assert_eq!(staker_one_info.balance, Uint128::from(2000u128));
    assert_eq!(staker_one_info.pending_rewards.len(), 2);
    for (asset, reward) in staker_one_info.pending_rewards {
        if asset == "uusd" {
            assert_eq!(reward, Uint128::from(50u128))
        }

        if asset == "uandr" {
            assert_eq!(reward, Uint128::from(20u128))
        }
    }

    let staker_two_info: StakerResponse = router
        .wrap()
        .query_wasm_smart(
            cw20_staking_addr,
            &mock_cw20_get_staker(staker_two.to_string()),
        )
        .unwrap();
    assert_eq!(staker_two_info.share, Uint128::from(2000u128));
    assert_eq!(staker_two_info.balance, Uint128::from(4000u128));
    assert_eq!(staker_two_info.pending_rewards.len(), 2);
    for (asset, reward) in staker_two_info.pending_rewards {
        if asset == "uusd" {
            assert_eq!(reward, Uint128::from(100u128))
        }

        if asset == "uandr" {
            assert_eq!(reward, Uint128::from(40u128))
        }
    }
}
