#![cfg(not(target_arch = "wasm32"))]

use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, MockAppContract};

use andromeda_testing::mock::mock_app;
use andromeda_testing::mock_builder::MockAndromedaBuilder;
use andromeda_validator_staking::mock::{
    mock_andromeda_validator_staking, mock_validator_staking_instantiate_msg, MockValidatorStaking,
};

// use andromeda_std::error::ContractError;
use andromeda_std::error::ContractError::Std;
use andromeda_testing::MockContract;
use cosmwasm_std::StdError::GenericErr;
use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Delegation, Uint128};

#[test]
fn test_validator_stake() {
    let mut router = mock_app(Some(vec!["TOKEN"]));

    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![("owner", vec![coin(1000, "TOKEN")])])
        .with_contracts(vec![
            ("app-contract", mock_andromeda_app()),
            ("validator-staking", mock_andromeda_validator_staking()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let validator_1 = router.api().addr_make("validator1");

    let validator_staking_init_msg = mock_validator_staking_instantiate_msg(
        validator_1.clone(),
        None,
        andr.kernel.addr().to_string(),
    );

    let validator_staking_component = AppComponent::new(
        "staking".to_string(),
        "validator-staking".to_string(),
        to_json_binary(&validator_staking_init_msg).unwrap(),
    );

    let app_components = vec![validator_staking_component.clone()];
    let app = MockAppContract::instantiate(
        andr.get_code_id(&mut router, "app-contract"),
        owner,
        &mut router,
        "Validator Staking App",
        app_components,
        andr.kernel.addr(),
        Some(owner.to_string()),
    );

    let validator_staking: MockValidatorStaking =
        app.query_ado_by_component_name(&router, validator_staking_component.name);

    let funds = vec![coin(1000, "TOKEN")];

    validator_staking
        .execute_stake(&mut router, owner.clone(), None, funds)
        .unwrap();

    let stake_info = validator_staking
        .query_staked_tokens(&router, None)
        .unwrap();
    assert_eq!(stake_info.validator, validator_1.to_string());

    // Testing when there is no reward to claim
    // TODO: These errors cant be downcast anymore?
    let _err = validator_staking
        .execute_claim_reward(&mut router, owner.clone(), Some(validator_1.clone()))
        .unwrap_err();
    // assert_eq!(may_err.unwrap(), &expected_err);

    // wait 1/2 year
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: router
            .block_info()
            .time
            .plus_seconds(60 * 60 * 24 * 365 / 2),
        chain_id: router.block_info().chain_id,
    });

    validator_staking
        .execute_claim_reward(&mut router, owner.clone(), Some(validator_1))
        .unwrap();

    // Default APR 10% by cw-multi-test -> StakingInfo
    // should now have 1000 * 10% / 2 - 0% commission = 50 tokens reward
    let contract_balance = router
        .wrap()
        .query_balance(validator_staking.addr(), "TOKEN")
        .unwrap();
    assert_eq!(contract_balance, coin(50, "TOKEN"));

    // Test unstake with invalid validator
    let _err = validator_staking
        .execute_unstake(
            &mut router,
            owner.clone(),
            Some(Addr::unchecked("fake_validator")),
            None,
        )
        .unwrap_err();
    // let _err = err.root_cause().downcast_ref::<ContractError>().unwrap();

    // let expected_err = ContractError::InvalidValidator {};
    // assert_eq!(err, &expected_err);

    // Test unstake from invalid owner
    let _err = validator_staking
        .execute_unstake(
            &mut router,
            Addr::unchecked("other"),
            Some(Addr::unchecked("fake_validator")),
            None,
        )
        .unwrap_err();
    // let _err = err.root_cause().downcast_ref::<ContractError>().unwrap();

    // let expected_err = ContractError::Unauthorized {};
    // assert_eq!(err, &expected_err);

    validator_staking
        .execute_unstake(&mut router, owner.clone(), None, None)
        .unwrap();

    // Test staked token query from undelegated validator
    let err = validator_staking
        .query_staked_tokens(&router, None)
        .unwrap_err();
    assert_eq!(
        err,
        Std(GenericErr {
            msg: "Querier contract error: InvalidDelegation".to_string()
        })
    );

    let unstaked_tokens = validator_staking.query_unstaked_tokens(&router).unwrap();
    let unbonding_period =
        unstaked_tokens[0].payout_at.seconds() - router.block_info().time.seconds();
    // Update block to payout period
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: router.block_info().time.plus_seconds(unbonding_period),
        chain_id: router.block_info().chain_id,
    });

    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: router.block_info().time.plus_seconds(1),
        chain_id: router.block_info().chain_id,
    });

    validator_staking
        .execute_withdraw_fund(&mut router, owner.clone())
        .unwrap();

    let owner_balance = router.wrap().query_balance(owner, "TOKEN").unwrap();
    assert_eq!(owner_balance, coin(1050, "TOKEN"));
}

#[test]
fn test_validator_stake_and_unstake_specific_amount() {
    let mut router = mock_app(Some(vec!["TOKEN"]));

    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![("owner", vec![coin(1000, "TOKEN")])])
        .with_contracts(vec![
            ("app-contract", mock_andromeda_app()),
            ("validator-staking", mock_andromeda_validator_staking()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let validator_1 = router.api().addr_make("validator1");

    let validator_staking_init_msg = mock_validator_staking_instantiate_msg(
        validator_1.clone(),
        None,
        andr.kernel.addr().to_string(),
    );

    let validator_staking_component = AppComponent::new(
        "staking".to_string(),
        "validator-staking".to_string(),
        to_json_binary(&validator_staking_init_msg).unwrap(),
    );

    let app_components = vec![validator_staking_component.clone()];
    let app = MockAppContract::instantiate(
        andr.get_code_id(&mut router, "app-contract"),
        owner,
        &mut router,
        "Validator Staking App",
        app_components,
        andr.kernel.addr(),
        Some(owner.to_string()),
    );

    let validator_staking: MockValidatorStaking =
        app.query_ado_by_component_name(&router, validator_staking_component.name);

    let funds = vec![coin(1000, "TOKEN")];

    validator_staking
        .execute_stake(&mut router, owner.clone(), None, funds)
        .unwrap();

    let stake_info = validator_staking
        .query_staked_tokens(&router, None)
        .unwrap();
    assert_eq!(stake_info.validator, validator_1.to_string());

    // Testing when there is no reward to claim
    let _err = validator_staking
        .execute_claim_reward(&mut router, owner.clone(), Some(validator_1.clone()))
        .unwrap_err();
    // assert_eq!(may_err.unwrap(), &expected_err);

    // wait 1/2 year
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: router
            .block_info()
            .time
            .plus_seconds(60 * 60 * 24 * 365 / 2),
        chain_id: router.block_info().chain_id,
    });

    validator_staking
        .execute_claim_reward(&mut router, owner.clone(), Some(validator_1))
        .unwrap();

    // Default APR 10% by cw-multi-test -> StakingInfo
    // should now have 1000 * 10% / 2 - 0% commission = 50 tokens reward
    let contract_balance = router
        .wrap()
        .query_balance(validator_staking.addr(), "TOKEN")
        .unwrap();
    assert_eq!(contract_balance, coin(50, "TOKEN"));

    // Test unstake with invalid validator
    let _err = validator_staking
        .execute_unstake(
            &mut router,
            owner.clone(),
            Some(Addr::unchecked("fake_validator")),
            None,
        )
        .unwrap_err();

    // Test unstake from invalid owner
    let _err = validator_staking
        .execute_unstake(
            &mut router,
            Addr::unchecked("other"),
            Some(Addr::unchecked("fake_validator")),
            None,
        )
        .unwrap_err();

    // Try unstaking an amount larger than staked
    validator_staking
        .execute_unstake(&mut router, owner.clone(), None, Some(Uint128::new(2_000)))
        .unwrap_err();

    // Try unstaking a zero amount
    validator_staking
        .execute_unstake(&mut router, owner.clone(), None, Some(Uint128::zero()))
        .unwrap_err();

    validator_staking
        .execute_unstake(&mut router, owner.clone(), None, Some(Uint128::new(200)))
        .unwrap();

    // Test staked token query from undelegated validator
    let delegation = validator_staking
        .query_staked_tokens(&router, None)
        .unwrap();
    assert_eq!(
        delegation,
        Delegation {
            delegator: Addr::unchecked(
                "andr12sjjp2n9epg6x8g4adaumlfp4ulaj5jvcu5rj29uk02zcdvrfkuqdfct9e"
            ),
            validator: "andr1qcxce9c4thzxnfmpr2dqnnlqea9ey35y7tnke37fymfcgzte0zwshp76a9"
                .to_string(),
            amount: coin(800_u128, "TOKEN")
        }
    );

    let unstaked_tokens = validator_staking.query_unstaked_tokens(&router).unwrap();
    let unbonding_period =
        unstaked_tokens[0].payout_at.seconds() - router.block_info().time.seconds();

    // Update block to payout period
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: router.block_info().time.plus_seconds(unbonding_period),
        chain_id: router.block_info().chain_id,
    });

    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: router.block_info().time.plus_seconds(1),
        chain_id: router.block_info().chain_id,
    });

    validator_staking
        .execute_withdraw_fund(&mut router, owner.clone())
        .unwrap();

    let owner_balance = router.wrap().query_balance(owner, "TOKEN").unwrap();
    assert_eq!(owner_balance, coin(250, "TOKEN"));
}
