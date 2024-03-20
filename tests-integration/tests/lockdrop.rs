use andromeda_cw20::mock::{mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_cw20_send};
use andromeda_lockdrop::mock::{
    mock_andromeda_lockdrop, mock_claim_rewards, mock_cw20_hook_increase_incentives,
    mock_deposit_native, mock_enable_claims, mock_lockdrop_instantiate_msg, mock_withdraw_native,
};
use andromeda_testing::mock::MockAndromeda;
use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::{App, Executor};

fn mock_app() -> App {
    App::new(|router, _api, storage| {
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked("user1"),
                [coin(500, "uusd")].to_vec(),
            )
            .unwrap();
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked("user2"),
                [coin(500, "uusd")].to_vec(),
            )
            .unwrap();
    })
}

fn mock_andromeda(app: &mut App, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

/// Test taken from audit report
#[test]
fn test_lockdrop() {
    let mut app = mock_app();
    let andr = mock_andromeda(&mut app, Addr::unchecked("owner"));
    let code = mock_andromeda_cw20();
    let cw_20_code_id = app.store_code(code);

    let cw20_init_msg = mock_cw20_instantiate_msg(
        None,
        "Token".to_owned(),
        "TOK".to_owned(),
        18u8,
        vec![Cw20Coin {
            amount: 100u128.into(),
            address: "owner".to_string(),
        }],
        None,
        None,
        andr.kernel_address.to_string(),
    );

    let cw20_incentives_address = app
        .instantiate_contract(
            cw_20_code_id,
            Addr::unchecked("owner"),
            &cw20_init_msg,
            &[],
            "Token",
            None,
        )
        .unwrap();

    let code = mock_andromeda_lockdrop();
    let lockdrop_code_id = app.store_code(code);
    let current_timestamp = app.block_info().time.seconds();

    let init_msg = mock_lockdrop_instantiate_msg(
        current_timestamp,
        100u64,
        50u64,
        cw20_incentives_address.to_string(),
        "uusd".to_string(),
        None,
        None,
        andr.kernel_address.to_string(),
    );

    let lockdrop_addr = app
        .instantiate_contract(
            lockdrop_code_id,
            Addr::unchecked("owner"),
            &init_msg,
            &[],
            "staking",
            None,
        )
        .unwrap();

    app.set_block(BlockInfo {
        time: app.block_info().time.plus_seconds(1),
        ..app.block_info()
    });

    let msg = mock_deposit_native();
    app.execute_contract(
        Addr::unchecked("user1"),
        lockdrop_addr.clone(),
        &msg,
        &[coin(500, "uusd")],
    )
    .unwrap();

    let msg = mock_deposit_native();
    app.execute_contract(
        Addr::unchecked("user2"),
        lockdrop_addr.clone(),
        &msg,
        &[coin(500, "uusd")],
    )
    .unwrap();

    let msg = mock_cw20_send(
        lockdrop_addr.to_string(),
        100u128.into(),
        to_json_binary(&mock_cw20_hook_increase_incentives()).unwrap(),
    );

    app.execute_contract(Addr::unchecked("owner"), cw20_incentives_address, &msg, &[])
        .unwrap();

    app.set_block(BlockInfo {
        time: app.block_info().time.plus_seconds(100),
        ..app.block_info()
    });

    //enable claims
    let msg = mock_enable_claims();
    app.execute_contract(Addr::unchecked("owner"), lockdrop_addr.clone(), &msg, &[])
        .unwrap();

    //claim

    let msg = mock_claim_rewards();
    app.execute_contract(Addr::unchecked("user1"), lockdrop_addr.clone(), &msg, &[])
        .unwrap();

    let msg = mock_claim_rewards();
    app.execute_contract(Addr::unchecked("user2"), lockdrop_addr.clone(), &msg, &[])
        .unwrap();

    let balance = app
        .wrap()
        .query_balance("user1", "uusd".to_string())
        .unwrap();

    assert_eq!(balance.amount, Uint128::zero());

    let msg = mock_withdraw_native(None);

    app.execute_contract(Addr::unchecked("user1"), lockdrop_addr, &msg, &[])
        .unwrap();

    let balance = app
        .wrap()
        .query_balance("user1", "uusd".to_string())
        .unwrap();

    assert_eq!(balance.amount, Uint128::from(500u128));
}
