use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, MockAppContract};

use andromeda_cw20::mock::{mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_minter, MockCW20};
use andromeda_kernel::KernelContract;
use andromeda_testing::{
    mock::{mock_app, MockApp},
    mock_builder::MockAndromedaBuilder,
    MockAndromeda, MockContract,
};

use andromeda_std::{
    amp::{AndrAddr, Recipient},
    os::{
        self,
        kernel::{ExecuteMsg, InstantiateMsg},
    },
};
use cosmwasm_std::{coin, to_json_binary, Binary, Coin, Decimal, Empty, StdAck, Uint128};

use andromeda_finance::splitter::{
    AddressPercent, Cw20HookMsg, ExecuteMsg as SplitterExecuteMsg,
    InstantiateMsg as SplitterInstantiateMsg,
};
use andromeda_splitter::mock::{
    mock_andromeda_splitter, mock_splitter_instantiate_msg, MockSplitter,
};
use cw20::Cw20Coin;
use cw_multi_test::Contract;
use rstest::{fixture, rstest};

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

// Cross chain test
use andromeda_adodb::ADODBContract;
use andromeda_economics::EconomicsContract;
use andromeda_splitter::SplitterContract;
use andromeda_vfs::VFSContract;
use cw_orch::prelude::*;
use cw_orch_interchain::{prelude::*, types::IbcPacketOutcome, InterchainEnv};
use ibc_relayer_types::core::ics24_host::identifier::PortId;
#[test]
fn test_splitter_cross_chain_recipient() {
    // Here `juno-1` is the chain-id and `juno` is the address prefix for this chain
    let sender = Addr::unchecked("sender_for_all_chains").into_string();
    let buyer = Addr::unchecked("buyer").into_string();

    let interchain = MockInterchainEnv::new(vec![
        ("juno", &sender),
        ("osmosis", &sender),
        // Dummy chain to create unequal ports to test counterparty denom properly
        ("cosmoshub", &sender),
    ]);

    let juno = interchain.get_chain("juno").unwrap();
    let osmosis = interchain.get_chain("osmosis").unwrap();
    juno.set_balance(sender.clone(), vec![Coin::new(100000000000000, "juno")])
        .unwrap();
    juno.set_balance(buyer.clone(), vec![Coin::new(100000000000000, "juno")])
        .unwrap();

    let kernel_juno = KernelContract::new(juno.clone());
    let vfs_juno = VFSContract::new(juno.clone());
    let adodb_juno = ADODBContract::new(juno.clone());
    let economics_juno = EconomicsContract::new(juno.clone());
    let splitter_juno = SplitterContract::new(juno.clone());

    let kernel_osmosis = KernelContract::new(osmosis.clone());
    let vfs_osmosis = VFSContract::new(osmosis.clone());
    let adodb_osmosis = ADODBContract::new(osmosis.clone());

    kernel_juno.upload().unwrap();
    vfs_juno.upload().unwrap();
    adodb_juno.upload().unwrap();
    economics_juno.upload().unwrap();
    splitter_juno.upload().unwrap();

    kernel_osmosis.upload().unwrap();
    vfs_osmosis.upload().unwrap();
    adodb_osmosis.upload().unwrap();

    let init_msg_juno = &InstantiateMsg {
        owner: None,
        chain_name: "juno".to_string(),
    };
    let init_msg_osmosis = &InstantiateMsg {
        owner: None,
        chain_name: "osmosis".to_string(),
    };

    kernel_juno.instantiate(init_msg_juno, None, None).unwrap();
    kernel_osmosis
        .instantiate(init_msg_osmosis, None, None)
        .unwrap();

    // Set up channel from juno to osmosis
    let channel_receipt = interchain
        .create_contract_channel(&kernel_juno, &kernel_osmosis, "andr-kernel-1", None)
        .unwrap();

    // After channel creation is complete, we get the channel id, which is necessary for ICA remote execution
    let juno_channel = channel_receipt
        .interchain_channel
        .get_chain("juno")
        .unwrap()
        .channel
        .unwrap();

    // Set up channel from osmosis to cosmoshub for ICS20 transfers so that channel-0 is used on osmosis
    // Later when we create channel with juno, channel-1 will be used on osmosis
    let _channel_receipt = interchain
        .create_channel(
            "osmosis",
            "cosmoshub",
            &PortId::transfer(),
            &PortId::transfer(),
            "ics20-1",
            None,
        )
        .unwrap();

    // Set up channel from juno to osmosis for ICS20 transfers
    let channel_receipt = interchain
        .create_channel(
            "juno",
            "osmosis",
            &PortId::transfer(),
            &PortId::transfer(),
            "ics20-1",
            None,
        )
        .unwrap();

    let channel = channel_receipt
        .interchain_channel
        .get_ordered_ports_from("juno")
        .unwrap();

    // After channel creation is complete, we get the channel id, which is necessary for ICA remote execution
    let _juno_channel_ics20 = channel_receipt
        .interchain_channel
        .get_chain("juno")
        .unwrap()
        .channel
        .unwrap();

    vfs_juno
        .instantiate(
            &os::vfs::InstantiateMsg {
                kernel_address: kernel_juno.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    vfs_osmosis
        .instantiate(
            &os::vfs::InstantiateMsg {
                kernel_address: kernel_osmosis.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    adodb_juno
        .instantiate(
            &os::adodb::InstantiateMsg {
                kernel_address: kernel_juno.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    adodb_juno
        .execute(
            &os::adodb::ExecuteMsg::Publish {
                code_id: 4,
                ado_type: "economics".to_string(),
                action_fees: None,
                version: "1.1.1".to_string(),
                publisher: None,
            },
            None,
        )
        .unwrap();

    adodb_juno
        .execute(
            &os::adodb::ExecuteMsg::Publish {
                code_id: splitter_juno.code_id().unwrap(),
                ado_type: "splitter".to_string(),
                action_fees: None,
                version: "2.3.0-b.1".to_string(),
                publisher: None,
            },
            None,
        )
        .unwrap();

    economics_juno
        .instantiate(
            &os::economics::InstantiateMsg {
                kernel_address: kernel_juno.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    kernel_juno
        .execute(
            &ExecuteMsg::UpsertKeyAddress {
                key: "economics".to_string(),
                value: economics_juno.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();

    adodb_osmosis
        .instantiate(
            &os::adodb::InstantiateMsg {
                kernel_address: kernel_osmosis.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    adodb_osmosis
        .execute(
            &os::adodb::ExecuteMsg::Publish {
                code_id: 2,
                ado_type: "counter".to_string(),
                action_fees: None,
                version: "1.0.2".to_string(),
                publisher: None,
            },
            None,
        )
        .unwrap();

    adodb_osmosis
        .execute(
            &os::adodb::ExecuteMsg::Publish {
                code_id: 6,
                ado_type: "economics".to_string(),
                action_fees: None,
                version: "1.1.1".to_string(),
                publisher: None,
            },
            None,
        )
        .unwrap();

    kernel_juno
        .execute(
            &ExecuteMsg::UpsertKeyAddress {
                key: "vfs".to_string(),
                value: vfs_juno.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();

    kernel_juno
        .execute(
            &ExecuteMsg::UpsertKeyAddress {
                key: "adodb".to_string(),
                value: adodb_juno.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();

    kernel_osmosis
        .execute(
            &ExecuteMsg::UpsertKeyAddress {
                key: "vfs".to_string(),
                value: vfs_osmosis.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();

    kernel_osmosis
        .execute(
            &ExecuteMsg::UpsertKeyAddress {
                key: "adodb".to_string(),
                value: adodb_osmosis.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();

    kernel_juno
        .execute(
            &ExecuteMsg::AssignChannels {
                ics20_channel_id: Some(channel.clone().0.channel.unwrap().to_string()),
                direct_channel_id: Some(juno_channel.to_string()),
                chain: "osmosis".to_string(),
                kernel_address: kernel_osmosis.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();

    kernel_osmosis
        .execute(
            &ExecuteMsg::AssignChannels {
                ics20_channel_id: Some(channel.0.channel.clone().unwrap().to_string()),
                direct_channel_id: Some(juno_channel.to_string()),
                chain: "juno".to_string(),
                kernel_address: kernel_juno.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();

    let recipient = "osmo1qzskhrca90qy2yjjxqzq4yajy842x7c50xq33d";
    println!(
        "osmosis kernel address: {}",
        kernel_osmosis.address().unwrap()
    );

    splitter_juno
        .instantiate(
            &SplitterInstantiateMsg {
                recipients: vec![
                    AddressPercent {
                        recipient: Recipient {
                            address: AndrAddr::from_string(format!("ibc://osmosis/{}", recipient)),
                            msg: None,
                            ibc_recovery_address: None,
                        },
                        percent: Decimal::from_ratio(Uint128::from(1u128), Uint128::from(2u128)),
                    },
                    AddressPercent {
                        recipient: Recipient {
                            address: AndrAddr::from_string(format!(
                                "ibc://osmosis/{}",
                                kernel_osmosis.address().unwrap().into_string()
                            )),
                            msg: None,
                            ibc_recovery_address: None,
                        },
                        percent: Decimal::from_ratio(Uint128::from(1u128), Uint128::from(2u128)),
                    },
                ],
                lock_time: None,
                kernel_address: kernel_osmosis.address().unwrap().into_string(),
                owner: None,
                default_recipient: None,
            },
            None,
            None,
        )
        .unwrap();

    // Send funds to splitter
    let splitter_juno_send_request = splitter_juno
        .execute(
            &SplitterExecuteMsg::Send { config: None },
            Some(&[Coin {
                denom: "juno".to_string(),
                amount: Uint128::new(200),
            }]),
        )
        .unwrap();

    let packet_lifetime = interchain
        .await_packets("juno", splitter_juno_send_request)
        .unwrap();

    let ibc_denom = format!("ibc/{}/{}", channel.1.channel.unwrap().as_str(), "juno");

    // For testing a successful outcome of the first packet sent out in the tx, you can use:
    if let IbcPacketOutcome::Success { .. } = &packet_lifetime.packets[0].outcome {
        // Packet has been successfully acknowledged and decoded, the transaction has gone through correctly
        // Check recipient balance
        let balances = osmosis
            .query_all_balances(kernel_osmosis.address().unwrap())
            .unwrap();
        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].denom, ibc_denom);
        assert_eq!(balances[0].amount.u128(), 200);
    } else {
        panic!("packet timed out");
        // There was a decode error or the packet timed out
        // Else the packet timed-out, you may have a relayer error or something is wrong in your application
    };

    // Register trigger address
    kernel_juno
        .execute(
            &ExecuteMsg::UpsertKeyAddress {
                key: "trigger_key".to_string(),
                value: sender.clone(),
            },
            None,
        )
        .unwrap();

    // Construct an Execute msg from the kernel on juno inteded for the splitter on osmosis
    let kernel_juno_trigger_request = kernel_juno
        .execute(
            &ExecuteMsg::TriggerRelay {
                packet_sequence: 1,
                channel_id: channel.0.channel.clone().unwrap().to_string(),
                packet_ack: to_json_binary(&StdAck::Success(Binary::default())).unwrap(),
            },
            None,
        )
        .unwrap();

    let packet_lifetime = interchain
        .await_packets("juno", kernel_juno_trigger_request)
        .unwrap();

    // For testing a successful outcome of the first packet sent out in the tx, you can use:
    if let IbcPacketOutcome::Success { .. } = &packet_lifetime.packets[0].outcome {
        // Packet has been successfully acknowledged and decoded, the transaction has gone through correctly

        // Check recipient balance after trigger execute msg
        let balances = osmosis.query_all_balances(recipient).unwrap();
        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].denom, ibc_denom);
        assert_eq!(balances[0].amount.u128(), 100);
    } else {
        panic!("packet timed out");
        // There was a decode error or the packet timed out
        // Else the packet timed-out, you may have a relayer error or something is wrong in your application
    };
}
