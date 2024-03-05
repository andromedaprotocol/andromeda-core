#![cfg(not(target_arch = "wasm32"))]

use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, MockApp};

use andromeda_validator_staking::mock::{
    mock_andromeda_validator_staking, mock_validator_staking_instantiate_msg, MockValidatorStaking,
};

use andromeda_std::error::ContractError;
use andromeda_std::error::ContractError::Std;
use andromeda_testing::{mock::MockAndromeda, MockContract};
use cosmwasm_std::StdError::GenericErr;
use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Decimal, Timestamp, Validator};
use cw_multi_test::App;

fn mock_app() -> App {
    App::new(|router, api, storage| {
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked("owner"),
                [coin(100000000000, "TOKEN"), coin(100000000000, "uandr")].to_vec(),
            )
            .unwrap();

        router
            .staking
            .add_validator(
                api,
                storage,
                &BlockInfo {
                    height: 0,
                    time: Timestamp::default(),
                    chain_id: "my-testnet".to_string(),
                },
                Validator {
                    address: "validator_1".to_string(),
                    commission: Decimal::zero(),
                    max_commission: Decimal::percent(20),
                    max_change_rate: Decimal::percent(1),
                },
            )
            .unwrap();

        router
            .staking
            .add_validator(
                api,
                storage,
                &BlockInfo {
                    height: 0,
                    time: Timestamp::default(),
                    chain_id: "my-testnet".to_string(),
                },
                Validator {
                    address: "validator_2".to_string(),
                    commission: Decimal::zero(),
                    max_commission: Decimal::percent(20),
                    max_change_rate: Decimal::percent(1),
                },
            )
            .unwrap();
    })
}

fn mock_andromeda(app: &mut App, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

#[test]
fn test_validator_stake() {
    let owner = Addr::unchecked("owner");
    let validator_1 = Addr::unchecked("validator_1");

    let mut router = mock_app();

    let andr = mock_andromeda(&mut router, owner.clone());

    andr.store_ado(&mut router, mock_andromeda_app(), "app");
    andr.store_ado(
        &mut router,
        mock_andromeda_validator_staking(),
        "validator-staking",
    );
    let validator_staking_init_msg = mock_validator_staking_instantiate_msg(
        validator_1.clone(),
        None,
        andr.kernel.addr().to_string(),
    );

    let validator_staking_component = AppComponent::new(
        "1".to_string(),
        "validator-staking".to_string(),
        to_json_binary(&validator_staking_init_msg).unwrap(),
    );

    let app_components = vec![validator_staking_component.clone()];
    let app = MockApp::instantiate(
        andr.get_code_id(&mut router, "app"),
        owner.clone(),
        &mut router,
        "Validator Staking App",
        app_components,
        andr.kernel.addr(),
        Some(owner.to_string()),
    );

    let validator_staking: MockValidatorStaking =
        app.query_ado_by_component_name(&router, validator_staking_component.name);

    // Set owner of the Validator Staking componenent as owner for testing purpose
    app.execute_claim_ownership(&mut router, owner.clone(), Some("1".to_string()))
        .unwrap();

    let funds = vec![coin(1000, "TOKEN")];

    validator_staking
        .execute_stake(&mut router, owner.clone(), None, funds)
        .unwrap();

    let stake_info = validator_staking
        .query_staked_tokens(&router, None)
        .unwrap();
    assert_eq!(stake_info.validator, validator_1.to_string());

    // Test unstake with invalid validator
    let err = validator_staking
        .execute_unstake(
            &mut router,
            owner.clone(),
            Some(Addr::unchecked("fake_validator")),
        )
        .unwrap_err();
    let err = err.root_cause().downcast_ref::<ContractError>().unwrap();

    let expected_err = ContractError::InvalidValidator {};
    assert_eq!(err, &expected_err);

    // Test unstake from invalid owner
    let err = validator_staking
        .execute_unstake(
            &mut router,
            Addr::unchecked("other"),
            Some(Addr::unchecked("fake_validator")),
        )
        .unwrap_err();
    let err = err.root_cause().downcast_ref::<ContractError>().unwrap();

    let expected_err = ContractError::Unauthorized {};
    assert_eq!(err, &expected_err);

    validator_staking
        .execute_unstake(&mut router, owner, None)
        .unwrap();

    let err = validator_staking
        .query_staked_tokens(&router, None)
        .unwrap_err();
    assert_eq!(
        err,
        Std(GenericErr {
            msg: "Querier contract error: InvalidDelegation".to_string()
        })
    );
}
