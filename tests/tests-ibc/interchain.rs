#![cfg(not(target_arch = "wasm32"))]
use andromeda_counter::CounterContract;
use andromeda_kernel::ack::make_ack_success;
use andromeda_math::counter::{
    CounterRestriction, GetCurrentAmountResponse, InstantiateMsg as CounterInstantiateMsg, State,
};

use andromeda_splitter::SplitterContract;
use andromeda_std::{
    amp::{
        messages::{AMPMsg, AMPMsgConfig},
        AndrAddr, Recipient,
    },
    os,
};

use andromeda_testing::{
    interchain::{ensure_packet_success, DEFAULT_SENDER},
    InterchainTestEnv,
};
use cosmwasm_std::{to_json_binary, Binary, Decimal, Uint128, CosmosMsg};
use cw_orch::prelude::*;
use cw_orch_interchain::prelude::*;

#[test]
fn test_kernel_ibc_execute_only() {
    let InterchainTestEnv {
        juno,
        osmosis,
        interchain,
        ..
    } = InterchainTestEnv::new();

    let counter_osmosis = CounterContract::new(osmosis.chain.clone());
    counter_osmosis.upload().unwrap();
    osmosis
        .aos
        .adodb
        .execute(
            &os::adodb::ExecuteMsg::Publish {
                code_id: counter_osmosis.code_id().unwrap(),
                ado_type: "counter".to_string(),
                action_fees: None,
                version: "1.0.2".to_string(),
                publisher: None,
            },
            None,
        )
        .unwrap();

    counter_osmosis
        .instantiate(
            &CounterInstantiateMsg {
                restriction: CounterRestriction::Public,
                initial_state: State {
                    initial_amount: None,
                    increase_amount: Some(1),
                    decrease_amount: None,
                },
                kernel_address: osmosis.aos.kernel.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();
    
    let kernel_juno_send_request = juno
        .aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::Send {
                message: AMPMsg {
                    recipient: AndrAddr::from_string(format!(
                        "ibc://osmosis/{}",
                        counter_osmosis.address().unwrap()
                    )),
                    message: to_json_binary(&andromeda_counter::mock::mock_execute_increment_msg())
                        .unwrap(),
                    funds: vec![],
                    config: AMPMsgConfig {
                        reply_on: cosmwasm_std::ReplyOn::Always,
                        exit_at_error: false,
                        gas_limit: None,
                        direct: true,
                        ibc_config: None,
                    },
                },
            },
            None,
        )
        .unwrap();
    
    println!("Kernel Juno Send Request: {:?}", &kernel_juno_send_request);
    let packet_lifetime = interchain
        .await_packets("juno-1", kernel_juno_send_request)
        .unwrap();

    ensure_packet_success(packet_lifetime);

    let current_count: GetCurrentAmountResponse = counter_osmosis
        .query(&andromeda_math::counter::QueryMsg::GetCurrentAmount {})
        .unwrap();
    assert_eq!(current_count.current_amount, 1);
}

#[test]
fn test_kernel_ibc_funds_only() {
    let InterchainTestEnv {
        juno,
        osmosis,
        interchain,
        ..
    } = InterchainTestEnv::new();

    let recipient = osmosis.chain.addr_make("recipient");

    let andr_recipient = AndrAddr::from_string(format!("ibc://osmosis/{}", recipient.to_string()));

    let message = AMPMsg::new(
        osmosis_recipient,
        Binary::default(),
        Some(vec![Coin {
            amount: Uint128::new(100000),
            denom: "ujuno".to_string(),
        }]),
    );

    let kernel_juno_send_request = juno
        .aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::Send { message },
            Some(&[Coin {
                amount: Uint128::new(100000),
                denom: "ujuno".to_string(),
            }]),
        )
        .unwrap();

    println!("Kernel Juno Send Request: {:?}", &kernel_juno_send_request);
    
    let packet_lifetime = interchain
        .await_packets("juno-1", kernel_juno_send_request)
        .unwrap();

    ensure_packet_success(packet_lifetime);

    let ibc_denom: String = format!(
        "ibc/{}/{}",
        osmosis.aos.get_aos_channel("juno").unwrap().direct.unwrap(),
        "juno"
    );

    let balances = osmosis
        .chain
        .query_all_balances(&osmosis.aos.kernel.address().unwrap())
        .unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].denom, ibc_denom);
    assert_eq!(balances[0].amount.u128(), 100);

    // Register trigger address
    juno.aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::UpsertKeyAddress {
                key: "trigger_key".to_string(),
                value: juno.chain.sender.to_string(),
            },
            None,
        )
        .unwrap();

    let packet_ack = make_ack_success();

    let channel_id = juno.aos.get_aos_channel("osmosis").unwrap().ics20.unwrap();
    // Construct an Execute msg from the kernel on juno inteded for the splitter on osmosis
    let kernel_juno_trigger_request = juno
        .aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::TriggerRelay {
                packet_sequence: 1,
                packet_ack,
                channel_id,
            },
            None,
        )
        .unwrap();

    let packet_lifetime = interchain
        .await_packets("juno", kernel_juno_trigger_request)
        .unwrap();
    ensure_packet_success(packet_lifetime);

    let balances = osmosis
        .chain
        .query_all_balances(&recipient)
        .unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].denom, ibc_denom);
    assert_eq!(balances[0].amount.u128(), 100);
}

#[test]
fn test_kernel_ibc_funds_and_execute_msg() {
    let InterchainTestEnv {
        juno,
        osmosis,
        interchain,
        ..
    } = InterchainTestEnv::new();

    let recipient = osmosis.chain.addr_make("recipient");

    let splitter_osmosis = SplitterContract::new(osmosis.chain.clone());
    splitter_osmosis.upload().unwrap();

    // This section covers the actions that take place after a successful ack from the ICS20 transfer is received
    // Let's instantiate a splitter
    splitter_osmosis
        .instantiate(
            &andromeda_finance::splitter::InstantiateMsg {
                recipients: vec![andromeda_finance::splitter::AddressPercent {
                    recipient: Recipient {
                        address: AndrAddr::from_string(recipient.to_string()),
                        msg: None,
                        ibc_recovery_address: None,
                    },
                    percent: Decimal::one(),
                }],
                default_recipient: None,
                lock_time: None,
                kernel_address: osmosis.aos.kernel.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();
    osmosis
        .aos
        .adodb
        .execute(
            &os::adodb::ExecuteMsg::Publish {
                code_id: splitter_osmosis.code_id().unwrap(),
                ado_type: "splitter".to_string(),
                action_fees: None,
                version: "1.0.0".to_string(),
                publisher: None,
            },
            None,
        )
        .unwrap();

    // Create the message to send to the kernel on juno
    let osmosis_recipient = AndrAddr::from_string(format!(
        "ibc://osmosis/{}",
        splitter_osmosis.address().unwrap()
    ));
    let message = AMPMsg::new(
        osmosis_recipient,
        to_json_binary(&andromeda_finance::splitter::ExecuteMsg::Send { config: None }).unwrap(),
        Some(vec![Coin {
            denom: "juno".to_string(),
            amount: Uint128::new(100),
        }]),
    );

    let kernel_juno_send_request = juno
        .aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::Send { message },
            Some(&[Coin {
                denom: "juno".to_string(),
                amount: Uint128::new(100),
            }]),
        )
        .unwrap();

    let packet_lifetime = interchain
        .await_packets("juno", kernel_juno_send_request)
        .unwrap();
    ensure_packet_success(packet_lifetime);

    // For testing a successful outcome of the first packet sent out in the tx, you can use:
    let ibc_denom = format!(
        "ibc/{}/{}",
        osmosis.aos.get_aos_channel("juno").unwrap().direct.unwrap(),
        "juno"
    );
    // Check kernel balance before trigger execute msg
    let balances = osmosis
        .chain
        .query_all_balances(&osmosis.aos.kernel.address().unwrap())
        .unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].denom, ibc_denom);
    assert_eq!(balances[0].amount.u128(), 100);

    // Register trigger address
    juno.aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::UpsertKeyAddress {
                key: "trigger_key".to_string(),
                value: juno.chain.sender.to_string(),
            },
            None,
        )
        .unwrap();

    let packet_ack = make_ack_success();

    let channel_id = juno.aos.get_aos_channel("osmosis").unwrap().ics20.unwrap();
    // Construct an Execute msg from the kernel on juno inteded for the splitter on osmosis
    let kernel_juno_splitter_request = juno
        .aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::TriggerRelay {
                packet_sequence: 1,
                packet_ack,
                channel_id,
            },
            None,
        )
        .unwrap();

    let packet_lifetime = interchain
        .await_packets("juno", kernel_juno_splitter_request)
        .unwrap();
    ensure_packet_success(packet_lifetime);

    // Check recipient balance after trigger execute msg
    let balances = osmosis
        .chain
        .query_all_balances(&recipient)
        .unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].denom, ibc_denom);
    assert_eq!(balances[0].amount.u128(), 100);
}
