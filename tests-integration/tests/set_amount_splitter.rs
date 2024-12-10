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

use andromeda_finance::set_amount_splitter::AddressAmount;
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
    cw20: Option<MockCW20>,
}

#[fixture]
fn wallets() -> Vec<(&'static str, Vec<Coin>)> {
    vec![
        ("owner", vec![]),
        ("buyer_one", vec![coin(1000000, "uandr")]),
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
    let buyer_one = andr.get_wallet("buyer_one");

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
        "set_amount_splitter".to_string(),
        "set_amount_splitter".to_string(),
        to_json_binary(&splitter_init_msg).unwrap(),
    );

    let mut app_components = vec![splitter_component.clone()];

    // Add cw20 components for test cases using cw20
    let cw20_component: Option<AppComponent> = match use_native_token {
        true => None,
        false => {
            let initial_balances = vec![Cw20Coin {
                address: buyer_one.to_string(),
                amount: Uint128::from(1000000u128),
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
            Some(cw20_component)
        }
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

    let cw20: Option<MockCW20> = match use_native_token {
        true => None,
        false => Some(app.query_ado_by_component_name(&router, cw20_component.unwrap().name)),
    };

    // Update splitter recipients to use cw20 if applicable
    if let Some(ref cw20) = cw20 {
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
fn test_successful_set_amount_splitter_native(#[with(true)] setup: TestCase) {
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
}

// #[test]
// fn test_splitter() {
//     let mut router = mock_app(None);
//     let andr = MockAndromedaBuilder::new(&mut router, "admin")
//         .with_wallets(vec![
//             ("owner", vec![coin(1000, "uandr")]),
//             ("recipient1", vec![]),
//             ("recipient2", vec![]),
//         ])
//         .with_contracts(vec![
//             ("app-contract", mock_andromeda_app()),
//             ("splitter", mock_andromeda_set_amount_splitter()),
//         ])
//         .build(&mut router);
//     let owner = andr.get_wallet("owner");
//     let recipient_1 = andr.get_wallet("recipient1");
//     let recipient_2 = andr.get_wallet("recipient2");

//     let app_code_id = andr.get_code_id(&mut router, "app-contract");

//     let splitter_recipients = vec![
//         AddressAmount {
//             recipient: Recipient::from_string(recipient_1.to_string()),
//             coins: coins(100_u128, "uandr"),
//         },
//         AddressAmount {
//             recipient: Recipient::from_string(recipient_2.to_string()),
//             coins: coins(50_u128, "uandr"),
//         },
//     ];

//     let splitter_init_msg = mock_set_amount_splitter_instantiate_msg(
//         splitter_recipients,
//         andr.kernel.addr().clone(),
//         None,
//         None,
//         None,
//     );
//     let splitter_app_component = AppComponent {
//         name: "splitter".to_string(),
//         component_type: ComponentType::new(splitter_init_msg),
//         ado_type: "splitter".to_string(),
//     };

//     let app_components = vec![splitter_app_component.clone()];
//     let app = MockAppContract::instantiate(
//         app_code_id,
//         owner,
//         &mut router,
//         "Splitter App",
//         app_components,
//         andr.kernel.addr(),
//         None,
//     );

//     let splitter: MockSetAmountSplitter =
//         app.query_ado_by_component_name(&router, splitter_app_component.name);

//     let token = coin(1000, "uandr");
//     splitter
//         .execute_send(&mut router, owner.clone(), &[token], None)
//         .unwrap();

//     let balance_1 = router.wrap().query_balance(recipient_1, "uandr").unwrap();
//     let balance_2 = router.wrap().query_balance(recipient_2, "uandr").unwrap();
//     let balance_owner = router.wrap().query_balance(owner, "uandr").unwrap();

//     assert_eq!(balance_1.amount, Uint128::from(100u128));
//     assert_eq!(balance_2.amount, Uint128::from(50u128));
//     // The owner sent 1000 but only 150 was needed. His account should be now worth 850
//     assert_eq!(balance_owner.amount, Uint128::from(850u128));
// }
