use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, mock_claim_ownership_msg, MockAppContract};
use andromeda_finance::{splitter::AddressPercent, timelock::EscrowConditionInput};
use andromeda_splitter::mock::{
    mock_andromeda_splitter, mock_splitter_instantiate_msg, mock_splitter_send_msg, MockSplitter,
};
use andromeda_std::{
    amp::Recipient,
    common::{expiration::Expiry, Milliseconds},
    error::ContractError,
};
use andromeda_testing::{
    mock::mock_app, mock_builder::MockAndromedaBuilder, mock_contract::MockContract,
};
use andromeda_timelock::mock::{
    mock_andromeda_timelock, mock_timelock_instantiate_msg, MockTimelock,
};
use cosmwasm_std::{coin, to_json_binary, Addr, Decimal, Uint128};
use cw_multi_test::Executor;
const ORIGINAL_BALANCE: u128 = 10_000;
#[test]
fn test_timelock() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![coin(ORIGINAL_BALANCE, "uandr")]),
            ("recipient", vec![]),
        ])
        .with_contracts(vec![
            ("timelock", mock_andromeda_timelock()),
            ("splitter", mock_andromeda_splitter()),
            ("app-contract", mock_andromeda_app()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let recipient = andr.get_wallet("recipient");
    // Generate App Components
    let timelock_init_msg = mock_timelock_instantiate_msg(andr.kernel.addr().to_string(), None);
    let timelock_component = AppComponent::new(
        "timelock".to_string(),
        "timelock".to_string(),
        to_json_binary(&timelock_init_msg).unwrap(),
    );

    let splitter_init_msg = mock_splitter_instantiate_msg(
        vec![AddressPercent {
            recipient: Recipient::new(recipient, None),
            percent: Decimal::percent(100),
        }],
        andr.kernel.addr().to_string(),
        None,
        None,
        None,
    );
    let splitter_component = AppComponent::new(
        "splitter".to_string(),
        "splitter".to_string(),
        to_json_binary(&splitter_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![timelock_component.clone(), splitter_component.clone()];
    let app = MockAppContract::instantiate(
        andr.get_code_id(&mut router, "app-contract"),
        owner,
        &mut router,
        "timelock App",
        app_components,
        andr.kernel.addr(),
        Some(owner.to_string()),
    );

    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(app.addr().clone()),
            &mock_claim_ownership_msg(None),
            &[],
        )
        .unwrap();

    let timelock: MockTimelock = app.query_ado_by_component_name(&router, timelock_component.name);

    // Test Case 1: Expiration from now

    // Hold Funds for 1 day in milliseconds
    let escrow_condition =
        EscrowConditionInput::Expiration(Expiry::FromNow(Milliseconds::from_seconds(86_400)));
    timelock
        .execute_hold_funds(
            &mut router,
            owner.clone(),
            &[coin(1000, "uandr")],
            Some(escrow_condition),
            None,
        )
        .unwrap();

    let owner_balance = router.wrap().query_balance(owner, "uandr").unwrap();
    assert_eq!(
        owner_balance.amount,
        Uint128::from(ORIGINAL_BALANCE - 1000u128)
    );

    // Let one hour elapse
    let block_time_plus_1h = router.block_info().time.plus_hours(1);
    router.update_block(|block| {
        block.time = block_time_plus_1h;
    });
    assert_eq!(block_time_plus_1h, router.block_info().time);

    // Try to release funds - should fail
    let err: ContractError = timelock
        .execute_release_funds(&mut router, owner.clone(), &[], None, None, None)
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::FundsAreLocked {});

    // Let two days elapse
    let block_time_plus_2d = router.block_info().time.plus_days(2);
    router.update_block(|block| {
        block.time = block_time_plus_2d;
    });

    // Ensure that the time has passed
    assert_eq!(block_time_plus_2d, router.block_info().time);

    // Release funds - should succeed
    timelock
        .execute_release_funds(&mut router, owner.clone(), &[], None, None, None)
        .unwrap();

    let owner_balance = router.wrap().query_balance(owner, "uandr").unwrap();
    assert_eq!(owner_balance.amount, Uint128::from(ORIGINAL_BALANCE));

    // Test Case 2: Expiration at specific time

    // Hold Funds for 1 day in milliseconds
    let escrow_condition = EscrowConditionInput::Expiration(Expiry::AtTime(
        Milliseconds::from_seconds(router.block_info().time.plus_days(1).seconds()),
    ));
    timelock
        .execute_hold_funds(
            &mut router,
            owner.clone(),
            &[coin(1000, "uandr")],
            Some(escrow_condition),
            None,
        )
        .unwrap();

    let owner_balance = router.wrap().query_balance(owner, "uandr").unwrap();
    assert_eq!(
        owner_balance.amount,
        Uint128::from(ORIGINAL_BALANCE - 1000u128)
    );

    // Let one hour elapse
    let block_time_plus_1h = router.block_info().time.plus_hours(1);
    router.update_block(|block| {
        block.time = block_time_plus_1h;
    });
    assert_eq!(block_time_plus_1h, router.block_info().time);

    // Try to release funds - should fail
    let err: ContractError = timelock
        .execute_release_funds(&mut router, owner.clone(), &[], None, None, None)
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::FundsAreLocked {});

    // Let two days elapse
    let block_time_plus_2d = router.block_info().time.plus_days(2);
    router.update_block(|block| {
        block.time = block_time_plus_2d;
    });

    // Ensure that the time has passed
    assert_eq!(block_time_plus_2d, router.block_info().time);

    // Release funds - should succeed
    timelock
        .execute_release_funds(&mut router, owner.clone(), &[], None, None, None)
        .unwrap();

    let owner_balance = router.wrap().query_balance(owner, "uandr").unwrap();
    assert_eq!(owner_balance.amount, Uint128::from(ORIGINAL_BALANCE));

    // Test Case 3: Minimum Funds

    let escrow_condition = EscrowConditionInput::MinimumFunds(vec![coin(1000, "uandr")]);
    timelock
        .execute_hold_funds(
            &mut router,
            owner.clone(),
            &[coin(100, "uandr")],
            Some(escrow_condition),
            None,
        )
        .unwrap();

    let owner_balance = router.wrap().query_balance(owner, "uandr").unwrap();
    assert_eq!(
        owner_balance.amount,
        Uint128::from(ORIGINAL_BALANCE - 100u128)
    );

    // Try to release funds - should fail
    let err: ContractError = timelock
        .execute_release_funds(&mut router, owner.clone(), &[], None, None, None)
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::FundsAreLocked {});

    let escrow_condition = EscrowConditionInput::MinimumFunds(vec![coin(1000, "uandr")]);
    timelock
        .execute_hold_funds(
            &mut router,
            owner.clone(),
            &[coin(900, "uandr")],
            Some(escrow_condition),
            None,
        )
        .unwrap();

    let owner_balance = router.wrap().query_balance(owner, "uandr").unwrap();
    assert_eq!(
        owner_balance.amount,
        Uint128::from(ORIGINAL_BALANCE - 1000u128)
    );

    // Release funds - should succeed
    timelock
        .execute_release_funds(&mut router, owner.clone(), &[], None, None, None)
        .unwrap();

    let owner_balance = router.wrap().query_balance(owner, "uandr").unwrap();
    assert_eq!(owner_balance.amount, Uint128::from(ORIGINAL_BALANCE));

    // Test case 4, recipient is a splitter contract and has a message attached to it
    let splitter: MockSplitter =
        app.query_ado_by_component_name(&router, splitter_component.name.clone());
    let splitter_send_msg = mock_splitter_send_msg(None);

    // Hold Funds for 1 day in milliseconds
    let escrow_condition =
        EscrowConditionInput::Expiration(Expiry::FromNow(Milliseconds::from_seconds(86_400)));
    timelock
        .execute_hold_funds(
            &mut router,
            owner.clone(),
            &[coin(1000, "uandr")],
            Some(escrow_condition),
            Some(Recipient::new(
                splitter.addr(),
                Some(to_json_binary(&splitter_send_msg).unwrap()),
            )),
        )
        .unwrap();

    // Let two days elapse
    let block_time_plus_2d = router.block_info().time.plus_days(2);
    router.update_block(|block| {
        block.time = block_time_plus_2d;
    });

    // Release funds - should succeed
    timelock
        .execute_release_funds(
            &mut router,
            owner.clone(),
            &[],
            // We get a NoFundsLocked error if we don't specify the recipient
            Some(splitter.addr().clone().into_string()),
            None,
            None,
        )
        .unwrap();

    let owner_balance = router.wrap().query_balance(owner, "uandr").unwrap();
    assert_eq!(
        owner_balance.amount,
        Uint128::from(ORIGINAL_BALANCE - 1000u128)
    );

    let recipient_balance = router.wrap().query_balance(recipient, "uandr").unwrap();
    assert_eq!(recipient_balance.amount, Uint128::from(1000u128));

    // Test case 5, recipient is a splitter contract and has NO message attached to it
    let splitter: MockSplitter = app.query_ado_by_component_name(&router, splitter_component.name);

    // Hold Funds for 1 day in milliseconds
    let escrow_condition =
        EscrowConditionInput::Expiration(Expiry::FromNow(Milliseconds::from_seconds(86_400)));
    timelock
        .execute_hold_funds(
            &mut router,
            owner.clone(),
            &[coin(1000, "uandr")],
            Some(escrow_condition),
            Some(Recipient::new(splitter.addr(), None)),
        )
        .unwrap();

    // Let two days elapse
    let block_time_plus_2d = router.block_info().time.plus_days(2);
    router.update_block(|block| {
        block.time = block_time_plus_2d;
    });

    // Release funds - should succeed
    timelock
        .execute_release_funds(
            &mut router,
            owner.clone(),
            &[],
            // We get a NoFundsLocked error if we don't specify the recipient
            Some(splitter.addr().clone().into_string()),
            None,
            None,
        )
        .unwrap();

    let owner_balance = router.wrap().query_balance(owner, "uandr").unwrap();
    assert_eq!(
        owner_balance.amount,
        Uint128::from(ORIGINAL_BALANCE - 2000u128)
    );

    let recipient_balance = router.wrap().query_balance(recipient, "uandr").unwrap();
    assert_eq!(recipient_balance.amount, Uint128::from(1000u128));

    let splitter_balance = router
        .wrap()
        .query_balance(splitter.addr(), "uandr")
        .unwrap();
    assert_eq!(splitter_balance.amount, Uint128::from(1000u128));
}
