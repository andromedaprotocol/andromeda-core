use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, mock_claim_ownership_msg, MockAppContract};
use andromeda_cw20::mock::{mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_minter, MockCW20};
use andromeda_std::{
    ado_base::rates::{LocalRate, LocalRateType, LocalRateValue, PercentRate, Rate},
    amp::Recipient,
};

use andromeda_testing::{
    mock::mock_app, mock_builder::MockAndromedaBuilder, mock_contract::MockContract,
};
use cosmwasm_std::{coin, to_json_binary, Addr, Decimal, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::Executor;

#[test]
fn test_cw20_with_rates() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![]),
            ("buyer_one", vec![coin(1000, "uandr")]),
            ("buyer_two", vec![coin(1000, "uandr")]),
            ("recipient_one", vec![]),
            ("recipient_two", vec![]),
        ])
        .with_contracts(vec![
            ("cw20", mock_andromeda_cw20()),
            ("app-contract", mock_andromeda_app()),
        ])
        .build(&mut router);
    let owner = andr.get_wallet("owner");
    let buyer_one = andr.get_wallet("buyer_one");
    let buyer_two = andr.get_wallet("buyer_two");
    let recipient_one = andr.get_wallet("recipient_one");

    // Generate App Components
    let initial_balances = vec![
        Cw20Coin {
            address: buyer_one.to_string(),
            amount: Uint128::from(1000u128),
        },
        Cw20Coin {
            address: buyer_two.to_string(),
            amount: Uint128::from(2000u128),
        },
        Cw20Coin {
            address: owner.to_string(),
            amount: Uint128::from(10000u128),
        },
    ];
    let buyer_two_original_balance = Uint128::from(2000u128);

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
    let cw20_component = AppComponent::new(
        "cw20".to_string(),
        "cw20".to_string(),
        to_json_binary(&cw20_init_msg).unwrap(),
    );

    // Create App
    let app_components = vec![cw20_component.clone()];
    let app = MockAppContract::instantiate(
        andr.get_code_id(&mut router, "app-contract"),
        owner,
        &mut router,
        "Auction App",
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

    let cw20: MockCW20 = app.query_ado_by_component_name(&router, cw20_component.name);

    // Set rates to SendFrom
    cw20.execute_add_rate(
        &mut router,
        owner.clone(),
        "TransferFrom".to_string(),
        Rate::Local(LocalRate {
            rate_type: LocalRateType::Deductive,
            recipient: Recipient::new(recipient_one, None),
            value: LocalRateValue::Percent(PercentRate {
                percent: Decimal::percent(10),
            }),
            description: None,
        }),
    )
    .unwrap();

    // Increase allowance for owner
    cw20.execute_increase_allowance(
        &mut router,
        owner.clone(),
        buyer_one.clone().into_string(),
        Uint128::new(100000),
    )
    .unwrap();

    // Execute SendFrom
    cw20.execute_transfer_from(
        &mut router,
        buyer_one.clone(),
        buyer_two,
        Uint128::new(10),
        owner.clone().into_string(),
    )
    .unwrap();
    // Rates are 10% , so we expect a balance of one for recipients one and two and the leftover 8 will be sent to buyer two
    let recip_one_balance = cw20.query_balance(&router, recipient_one);
    assert_eq!(Uint128::one(), recip_one_balance);

    let buyer_two_balance = cw20.query_balance(&router, buyer_two);
    assert_eq!(
        buyer_two_original_balance
            .checked_add(Uint128::new(9))
            .unwrap(),
        buyer_two_balance
    );
}
