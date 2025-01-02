use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, MockAppContract};

use andromeda_cw20::mock::{mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_minter, MockCW20};
use andromeda_testing::{
    mock::{mock_app, MockApp},
    mock_builder::MockAndromedaBuilder,
    MockAndromeda, MockContract,
};

use andromeda_std::amp::Recipient;
use cosmwasm_std::{coin, coins, to_json_binary, Coin, Empty, Uint128};

use andromeda_finance::set_amount_splitter::{AddressAmount, Cw20HookMsg};
use andromeda_set_amount_splitter::mock::{
    mock_andromeda_set_amount_splitter, mock_set_amount_splitter_instantiate_msg,
    MockSetAmountSplitter,
};
use cw20::Cw20Coin;
use cw_multi_test::Contract;
use rstest::{fixture, rstest};

struct TestCase {
    router: MockApp,
    andr: MockAndromeda,
    splitter: MockSetAmountSplitter,
    cw20: MockCW20,
}

#[fixture]
fn wallets() -> Vec<(&'static str, Vec<Coin>)> {
    vec![
        ("owner", vec![coin(1000000, "uandr")]),
        ("recipient1", vec![]),
        ("recipient2", vec![]),
    ]
}

#[fixture]
fn contracts() -> Vec<(&'static str, Box<dyn Contract<Empty>>)> {
    vec![
        ("cw20", mock_andromeda_cw20()),
        ("set-amount-splitter", mock_andromeda_set_amount_splitter()),
        ("app-contract", mock_andromeda_app()),
    ]
}

#[fixture]
fn setup(
    #[default(true)] use_native_token: bool,
    wallets: Vec<(&'static str, Vec<Coin>)>,
    contracts: Vec<(&'static str, Box<dyn Contract<Empty>>)>,
) -> TestCase {
    let mut router = mock_app(None);

    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(wallets)
        .with_contracts(contracts)
        .build(&mut router);

    let owner = andr.get_wallet("owner");

    // Prepare Splitter component which can be used as a withdrawal address for some test cases
    let recipient_1 = andr.get_wallet("recipient1");
    let recipient_2 = andr.get_wallet("recipient2");

    let splitter_recipients = vec![
        AddressAmount {
            recipient: Recipient::from_string(recipient_1.to_string()),
            coins: coins(100, "uandr"),
        },
        AddressAmount {
            recipient: Recipient::from_string(recipient_2.to_string()),
            coins: coins(100, "uandr"),
        },
    ];
    let splitter_init_msg = mock_set_amount_splitter_instantiate_msg(
        splitter_recipients,
        andr.kernel.addr().clone(),
        None,
        None,
        None,
    );
    let splitter_component = AppComponent::new(
        "set-amount-splitter".to_string(),
        "set-amount-splitter".to_string(),
        to_json_binary(&splitter_init_msg).unwrap(),
    );

    let mut app_components = vec![splitter_component.clone()];

    // Add cw20 components for test cases using cw20
    let cw20_component: AppComponent = {
        let initial_balances = vec![Cw20Coin {
            address: owner.to_string(),
            amount: Uint128::from(1_000_000u128),
        }];

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
        app_components.push(cw20_component.clone());
        cw20_component
    };

    let app = MockAppContract::instantiate(
        andr.get_code_id(&mut router, "app-contract"),
        owner,
        &mut router,
        "Set Amount Splitter App",
        app_components.clone(),
        andr.kernel.addr(),
        Some(owner.to_string()),
    );

    let splitter: MockSetAmountSplitter =
        app.query_ado_by_component_name(&router, splitter_component.name);

    let cw20: MockCW20 = app.query_ado_by_component_name(&router, cw20_component.name);

    // Update splitter recipients to use cw20 if applicable
    if !use_native_token {
        let cw20_addr = cw20.addr();
        let splitter_recipients = vec![
            AddressAmount {
                recipient: Recipient::from_string(recipient_1.to_string()),
                coins: coins(100, cw20_addr.clone()),
            },
            AddressAmount {
                recipient: Recipient::from_string(recipient_2.to_string()),
                coins: coins(100, cw20_addr.clone()),
            },
        ];

        splitter
            .execute_update_recipients(&mut router, owner.clone(), &[], splitter_recipients)
            .unwrap();
    }
    TestCase {
        router,
        andr,
        splitter,
        cw20,
    }
}

#[rstest]
fn test_successful_set_amount_splitter_native(setup: TestCase) {
    let TestCase {
        mut router,
        andr,
        splitter,
        ..
    } = setup;

    let owner = andr.get_wallet("owner");

    splitter
        .execute_send(&mut router, owner.clone(), &[coin(1000, "uandr")], None)
        .unwrap();

    assert_eq!(
        router
            .wrap()
            .query_balance(andr.get_wallet("recipient1"), "uandr")
            .unwrap()
            .amount,
        Uint128::from(100u128)
    );
    assert_eq!(
        router
            .wrap()
            .query_balance(andr.get_wallet("recipient2"), "uandr")
            .unwrap()
            .amount,
        Uint128::from(100u128)
    );
}

#[rstest]
fn test_successful_set_amount_splitter_cw20_with_remainder(#[with(false)] setup: TestCase) {
    let TestCase {
        mut router,
        andr,
        splitter,
        cw20,
    } = setup;

    let owner = andr.get_wallet("owner");

    let hook_msg = Cw20HookMsg::Send { config: None };

    cw20.execute_send(
        &mut router,
        owner.clone(),
        splitter.addr(),
        Uint128::new(1000),
        &hook_msg,
    )
    .unwrap();

    let cw20_balance = cw20.query_balance(&router, andr.get_wallet("recipient1"));
    assert_eq!(cw20_balance, Uint128::from(100u128));
    let cw20_balance = cw20.query_balance(&router, andr.get_wallet("recipient2"));
    assert_eq!(cw20_balance, Uint128::from(100u128));
    let cw20_balance = cw20.query_balance(&router, owner);
    assert_eq!(cw20_balance, Uint128::from(1_000_000u128 - 200u128));
}

#[rstest]
fn test_successful_set_amount_splitter_cw20_without_remainder(#[with(false)] setup: TestCase) {
    let TestCase {
        mut router,
        andr,
        splitter,
        cw20,
    } = setup;

    let owner = andr.get_wallet("owner");

    let hook_msg = Cw20HookMsg::Send { config: None };

    cw20.execute_send(
        &mut router,
        owner.clone(),
        splitter.addr(),
        Uint128::new(200),
        &hook_msg,
    )
    .unwrap();

    let cw20_balance = cw20.query_balance(&router, andr.get_wallet("recipient1"));
    assert_eq!(cw20_balance, Uint128::from(100u128));
    let cw20_balance = cw20.query_balance(&router, andr.get_wallet("recipient2"));
    assert_eq!(cw20_balance, Uint128::from(100u128));
    let cw20_balance = cw20.query_balance(&router, owner);
    assert_eq!(cw20_balance, Uint128::from(1_000_000u128 - 200u128));
}
