#![cfg(not(target_arch = "wasm32"))]
use andromeda_counter::CounterContract;
use andromeda_data_storage::counter::{
    CounterRestriction, GetCurrentAmountResponse, InstantiateMsg as CounterInstantiateMsg, State,
};

use andromeda_std::{
    amp::{
        messages::{AMPMsg, AMPMsgConfig},
        AndrAddr,
    },
    os::{
        self,
        kernel::{AcknowledgementMsg, SendMessageWithFundsResponse},
    },
};
use andromeda_testing::{interchain::DEFAULT_SENDER, InterchainTestEnv};
use cosmwasm_std::{
    to_json_binary, Binary, IbcAcknowledgement, IbcEndpoint, IbcPacket, IbcPacketAckMsg,
    IbcTimeout, Timestamp, Uint128,
};
use cw_orch::prelude::*;
use cw_orch_interchain::prelude::*;
use cw_orch_interchain::types::IbcPacketOutcome;

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

    let packet_lifetime = interchain
        .await_packets("juno", kernel_juno_send_request)
        .unwrap();

    assert!(matches!(
        packet_lifetime.packets[0].outcome,
        IbcPacketOutcome::Success { .. }
    ));

    let current_count: GetCurrentAmountResponse = counter_osmosis
        .query(&andromeda_data_storage::counter::QueryMsg::GetCurrentAmount {})
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

    let recipient = "osmo1qzskhrca90qy2yjjxqzq4yajy842x7c50xq33d";

    let kernel_juno_send_request = juno
        .aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::Send {
                message: AMPMsg {
                    recipient: AndrAddr::from_string(format!("ibc://osmosis/{}", recipient)),
                    message: Binary::default(),
                    funds: vec![Coin {
                        denom: "juno".to_string(),
                        amount: Uint128::new(100),
                    }],
                    config: AMPMsgConfig {
                        reply_on: cosmwasm_std::ReplyOn::Always,
                        exit_at_error: false,
                        gas_limit: None,
                        direct: true,
                        ibc_config: None,
                    },
                },
            },
            Some(&[Coin {
                denom: "juno".to_string(),
                amount: Uint128::new(100),
            }]),
        )
        .unwrap();

    let packet_lifetime = interchain
        .await_packets("juno", kernel_juno_send_request)
        .unwrap();

    let ibc_denom: String = format!(
        "ibc/{}/{}",
        osmosis.aos.get_aos_channel("juno").unwrap().direct.unwrap(),
        "juno"
    );

    // For testing a successful outcome of the first packet sent out in the tx, you can use:
    if let IbcPacketOutcome::Success { .. } = &packet_lifetime.packets[0].outcome {
        // Packet has been successfully acknowledged and decoded, the transaction has gone through correctly
        // Check recipient balance
        let balances = osmosis
            .chain
            .query_all_balances(osmosis.aos.kernel.address().unwrap())
            .unwrap();
        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].denom, ibc_denom);
        assert_eq!(balances[0].amount.u128(), 100);
    } else {
        panic!("packet timed out");
        // There was a decode error or the packet timed out
        // Else the packet timed-out, you may have a relayer error or something is wrong in your application
    };

    // Register trigger address
    juno.aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::UpsertKeyAddress {
                key: "trigger_key".to_string(),
                value: DEFAULT_SENDER.to_string(),
            },
            None,
        )
        .unwrap();

    // Construct an Execute msg from the kernel on juno inteded for the splitter on osmosis
    let kernel_juno_trigger_request = juno
        .aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::TriggerRelay {
                packet_sequence: "1".to_string(),
                packet_ack_msg: IbcPacketAckMsg::new(
                    IbcAcknowledgement::new(
                        to_json_binary(&AcknowledgementMsg::<SendMessageWithFundsResponse>::Ok(
                            SendMessageWithFundsResponse {},
                        ))
                        .unwrap(),
                    ),
                    IbcPacket::new(
                        Binary::default(),
                        IbcEndpoint {
                            port_id: "port_id".to_string(),
                            channel_id: "channel_id".to_string(),
                        },
                        IbcEndpoint {
                            port_id: "port_id".to_string(),
                            channel_id: "channel_id".to_string(),
                        },
                        1,
                        IbcTimeout::with_timestamp(Timestamp::from_seconds(1)),
                    ),
                    Addr::unchecked("relayer"),
                ),
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
        let balances = osmosis.chain.query_all_balances(recipient).unwrap();
        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].denom, ibc_denom);
        assert_eq!(balances[0].amount.u128(), 100);
    } else {
        panic!("packet timed out");
        // There was a decode error or the packet timed out
        // Else the packet timed-out, you may have a relayer error or something is wrong in your application
    };
}
