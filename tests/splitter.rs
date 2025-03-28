use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, MockAppContract};

use andromeda_cw20::mock::{mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_minter, MockCW20};
use andromeda_testing::{
    mock::{mock_app, MockApp},
    mock_builder::MockAndromedaBuilder,
    MockAndromeda, MockContract,
};

use andromeda_std::{
    amp::{AndrAddr, Recipient},
    os::{
        self,
    },
};
use cw_orch_interchain::core::InterchainEnv;
use cosmwasm_std::{coin, to_json_binary, Coin, Decimal, Empty, Uint128};

use andromeda_finance::splitter::{
    AddressPercent, Cw20HookMsg,
};
use andromeda_splitter::mock::{
    mock_andromeda_splitter, mock_splitter_instantiate_msg, MockSplitter,
};
// Cross chain test

use andromeda_splitter::SplitterContract;
use cw_orch::{mock::cw_multi_test::ibc::types::keccak256, prelude::*};
use cw20::Cw20Coin;
use cw_multi_test::Contract;
use rstest::{fixture, rstest};

use andromeda_testing::{interchain::ensure_packet_success, InterchainTestEnv};
use andromeda_kernel::ack::make_ack_success;

struct TestCase {
    router: MockApp,
    andr: MockAndromeda,
    splitter: MockSplitter,
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
        ("splitter", mock_andromeda_splitter()),
        ("app-contract", mock_andromeda_app()),
    ]
}

#[fixture]
fn setup(
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
        AddressPercent {
            recipient: Recipient::from_string(recipient_1.to_string()),
            percent: Decimal::from_ratio(Uint128::from(2u128), Uint128::from(10u128)),
        },
        AddressPercent {
            recipient: Recipient::from_string(recipient_2.to_string()),
            percent: Decimal::from_ratio(Uint128::from(8u128), Uint128::from(10u128)),
        },
    ];
    let splitter_init_msg = mock_splitter_instantiate_msg(
        splitter_recipients,
        andr.kernel.addr().clone(),
        None,
        None,
        None,
    );
    let splitter_component = AppComponent::new(
        "splitter".to_string(),
        "splitter".to_string(),
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

    let splitter: MockSplitter = app.query_ado_by_component_name(&router, splitter_component.name);

    let cw20: MockCW20 = app.query_ado_by_component_name(&router, cw20_component.name);

    TestCase {
        router,
        andr,
        splitter,
        cw20,
    }
}

#[rstest]
fn test_successful_fixed_amount_splitter_without_remainder_native(setup: TestCase) {
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
        Uint128::from(200u128)
    );
    assert_eq!(
        router
            .wrap()
            .query_balance(andr.get_wallet("recipient2"), "uandr")
            .unwrap()
            .amount,
        Uint128::from(800u128)
    );
}

#[rstest]
fn test_successful_fixed_amount_splitter_with_remainder_native(setup: TestCase) {
    let TestCase {
        mut router,
        andr,
        splitter,
        ..
    } = setup;

    let owner = andr.get_wallet("owner");

    let splitter_recipients = vec![
        AddressPercent {
            recipient: Recipient::from_string(andr.get_wallet("recipient1").to_string()),
            percent: Decimal::from_ratio(Uint128::from(1u128), Uint128::from(10u128)),
        },
        AddressPercent {
            recipient: Recipient::from_string(andr.get_wallet("recipient2").to_string()),
            percent: Decimal::from_ratio(Uint128::from(1u128), Uint128::from(10u128)),
        },
    ];

    splitter
        .execute_update_recipients(&mut router, owner.clone(), &[], splitter_recipients)
        .unwrap();

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
    assert_eq!(
        router
            .wrap()
            .query_balance(andr.get_wallet("owner"), "uandr")
            .unwrap()
            .amount,
        Uint128::from(1_000_000u128 - 200u128)
    );
}

#[rstest]
fn test_successful_fixed_amount_splitter_cw20_without_remainder(setup: TestCase) {
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
    assert_eq!(cw20_balance, Uint128::from(200u128));
    let cw20_balance = cw20.query_balance(&router, andr.get_wallet("recipient2"));
    assert_eq!(cw20_balance, Uint128::from(800u128));
    let cw20_balance = cw20.query_balance(&router, owner);
    assert_eq!(cw20_balance, Uint128::from(1_000_000u128 - 1000u128));
}

#[rstest]
fn test_successful_fixed_amount_splitter_cw20_with_remainder(setup: TestCase) {
    let TestCase {
        mut router,
        andr,
        splitter,
        cw20,
    } = setup;

    let owner = andr.get_wallet("owner");

    let splitter_recipients = vec![
        AddressPercent {
            recipient: Recipient::from_string(andr.get_wallet("recipient1").to_string()),
            percent: Decimal::from_ratio(Uint128::from(1u128), Uint128::from(10u128)),
        },
        AddressPercent {
            recipient: Recipient::from_string(andr.get_wallet("recipient2").to_string()),
            percent: Decimal::from_ratio(Uint128::from(1u128), Uint128::from(10u128)),
        },
    ];

    splitter
        .execute_update_recipients(&mut router, owner.clone(), &[], splitter_recipients)
        .unwrap();

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



#[test]
fn test_splitter_cross_chain_recipient() {
    // Initialize the interchain test environment which already has configured chains
    let InterchainTestEnv {
        juno,
        osmosis,
        interchain,
        ..
    } = InterchainTestEnv::new();

    // Create a recipient on osmosis
    let recipient = osmosis.chain.addr_make("recipient");

    // Upload and instantiate splitter on juno chain
    let splitter_juno = SplitterContract::new(juno.chain.clone());
    splitter_juno.upload().unwrap();

    // Register the splitter in ADODB
    juno.aos.adodb
        .execute(
            &os::adodb::ExecuteMsg::Publish {
                code_id: splitter_juno.code_id().unwrap(),
                ado_type: "splitter".to_string(),
                action_fees: None,
                version: "2.3.0".to_string(),
                publisher: None,
            },
            &[],
        )
        .unwrap();

    // Instantiate the splitter with two recipients: the specified recipient and the kernel on osmosis
    splitter_juno
        .instantiate(
            &andromeda_finance::splitter::InstantiateMsg {
                recipients: vec![
                    andromeda_finance::splitter::AddressPercent {
                        recipient: Recipient {
                            address: AndrAddr::from_string(format!("ibc://osmosis/{}", recipient)),
                            msg: None,
                            ibc_recovery_address: None,
                        },
                        percent: Decimal::from_ratio(Uint128::from(1u128), Uint128::from(2u128)),
                    },
                    andromeda_finance::splitter::AddressPercent {
                        recipient: Recipient {
                            address: AndrAddr::from_string(format!(
                                "ibc://osmosis/{}",
                                osmosis.aos.kernel.address().unwrap().into_string()
                            )),
                            msg: None,
                            ibc_recovery_address: None,
                        },
                        percent: Decimal::from_ratio(Uint128::from(1u128), Uint128::from(2u128)),
                    },
                ],
                lock_time: None,
                kernel_address: juno.aos.kernel.address().unwrap().into_string(),
                owner: None,
                default_recipient: None,
            },
            None,
            &[],
        )
        .unwrap();

    
    // Send funds to splitter
    let splitter_juno_send_request = splitter_juno
        .execute(
            &andromeda_finance::splitter::ExecuteMsg::Send { config: None },
            &[Coin {
                denom: juno.denom.clone(),
                amount: Uint128::new(200),
            }],
        )
        .unwrap();

    // Wait for packets to be processed
    let packet_lifetime = interchain
        .await_packets(&juno.chain_id, splitter_juno_send_request)
        .unwrap();

    // Using proper IBC denom based on the channel
    let ibc_channel = osmosis.aos.get_aos_channel("juno").unwrap().ics20.unwrap();
    let ibc_denom = format!("{}/{}", ibc_channel, juno.denom);
    let expected_denom = format!("ibc/{}", hex::encode(keccak256(ibc_denom.as_bytes())));


    // Check if the packet was successful and verify balances
    ensure_packet_success(packet_lifetime);
    
    // Check that the kernel on osmosis received funds
    let balances = osmosis
        .chain
        .query_all_balances(&osmosis.aos.kernel.address().unwrap())
        .unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].denom, expected_denom);
    assert_eq!(balances[0].amount.u128(), 200);

    // Register trigger address
    juno.aos.kernel
        .execute(
            &os::kernel::ExecuteMsg::UpsertKeyAddress {
                key: "trigger_key".to_string(),
                value: juno.chain.sender.to_string(),
            },
            &[],
        )
        .unwrap();

    // Create packet ack for trigger
    let packet_ack = make_ack_success();
    let channel_id = juno.aos.get_aos_channel("osmosis").unwrap().ics20.unwrap();
    
    // Trigger relay to complete the transaction
    let kernel_juno_trigger_request = juno.aos.kernel
        .execute(
            &os::kernel::ExecuteMsg::TriggerRelay {
                packet_sequence: 1,
                packet_ack,
                channel_id,
            },
            &[],
        )
        .unwrap();

    // Wait for trigger packet to be processed
    let packet_lifetime = interchain
        .await_packets(&juno.chain_id, kernel_juno_trigger_request)
        .unwrap();
    
    // Verify the trigger was successful
    ensure_packet_success(packet_lifetime);

    // Check recipient balance after trigger execute msg
    let balances = osmosis.chain.query_all_balances(&recipient).unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].denom, expected_denom);
    assert_eq!(balances[0].amount.u128(), 100);
}
