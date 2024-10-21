#![cfg(not(target_arch = "wasm32"))]
use andromeda_adodb::ADODBContract;
use andromeda_counter::CounterContract;
use andromeda_data_storage::counter::{
    CounterRestriction, ExecuteMsg as CounterExecuteMsg, GetCurrentAmountResponse,
    InstantiateMsg as CounterInstantiateMsg, State,
};
use andromeda_economics::EconomicsContract;
use andromeda_finance::splitter::{
    AddressPercent, ExecuteMsg as SplitterExecuteMsg, InstantiateMsg as SplitterInstantiateMsg,
};

use andromeda_kernel::KernelContract;
use andromeda_splitter::SplitterContract;
use andromeda_std::{
    amp::{
        messages::{AMPMsg, AMPMsgConfig},
        AndrAddr, Recipient,
    },
    os::{
        self,
        kernel::{AcknowledgementMsg, ExecuteMsg, InstantiateMsg, SendMessageWithFundsResponse},
    },
};
use andromeda_vfs::VFSContract;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Decimal, IbcAcknowledgement, IbcEndpoint, IbcPacket,
    IbcPacketAckMsg, IbcTimeout, Timestamp, Uint128,
};
use cw_orch::prelude::*;
use cw_orch_interchain::{prelude::*, types::IbcPacketOutcome, InterchainEnv};
use ibc_relayer_types::core::ics24_host::identifier::PortId;

#[test]
fn test_kernel_ibc_execute_only() {
    // Here `juno-1` is the chain-id and `juno` is the address prefix for this chain
    let sender = Addr::unchecked("sender_for_all_chains").into_string();

    let interchain = MockInterchainEnv::new(vec![("juno", &sender), ("osmosis", &sender)]);

    let juno = interchain.get_chain("juno").unwrap();
    let osmosis = interchain.get_chain("osmosis").unwrap();

    juno.set_balance(sender.clone(), vec![Coin::new(100000000000000, "juno")])
        .unwrap();

    let kernel_juno = KernelContract::new(juno.clone());
    let vfs_juno = VFSContract::new(juno.clone());
    let kernel_osmosis = KernelContract::new(osmosis.clone());
    let counter_osmosis = CounterContract::new(osmosis.clone());
    let vfs_osmosis = VFSContract::new(osmosis.clone());
    let adodb_osmosis = ADODBContract::new(osmosis.clone());

    kernel_juno.upload().unwrap();
    vfs_juno.upload().unwrap();
    kernel_osmosis.upload().unwrap();
    counter_osmosis.upload().unwrap();
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

    kernel_juno
        .execute(
            &ExecuteMsg::UpsertKeyAddress {
                key: "vfs".to_string(),
                value: vfs_juno.address().unwrap().into_string(),
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
                ics20_channel_id: None,
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
                ics20_channel_id: None,
                direct_channel_id: Some(juno_channel.to_string()),
                chain: "juno".to_string(),
                kernel_address: kernel_juno.address().unwrap().into_string(),
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
                kernel_address: kernel_osmosis.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();
    let kernel_juno_send_request = kernel_juno
        .execute(
            &ExecuteMsg::Send {
                message: AMPMsg {
                    recipient: AndrAddr::from_string(format!(
                        "ibc://osmosis/{}",
                        counter_osmosis.address().unwrap()
                    )),
                    message: to_json_binary(&CounterExecuteMsg::Increment {}).unwrap(),
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

    // For testing a successful outcome of the first packet sent out in the tx, you can use:
    if let IbcPacketOutcome::Success { .. } = &packet_lifetime.packets[0].outcome {
        // Packet has been successfully acknowledged and decoded, the transaction has gone through correctly
    } else {
        panic!("packet timed out");
        // There was a decode error or the packet timed out
        // Else the packet timed-out, you may have a relayer error or something is wrong in your application
    };

    let current_count: GetCurrentAmountResponse = counter_osmosis
        .query(&andromeda_data_storage::counter::QueryMsg::GetCurrentAmount {})
        .unwrap();
    assert_eq!(current_count.current_amount, 1);
}

#[test]
fn test_kernel_ibc_funds_only() {
    // Here `juno-1` is the chain-id and `juno` is the address prefix for this chain
    let sender = Addr::unchecked("sender_for_all_chains").into_string();

    let interchain = MockInterchainEnv::new(vec![("juno", &sender), ("osmosis", &sender)]);

    let juno = interchain.get_chain("juno").unwrap();
    let osmosis = interchain.get_chain("osmosis").unwrap();

    juno.set_balance(sender.clone(), vec![Coin::new(100000000000000, "juno")])
        .unwrap();

    let kernel_juno = KernelContract::new(juno.clone());
    let vfs_juno = VFSContract::new(juno.clone());
    let kernel_osmosis = KernelContract::new(osmosis.clone());
    let counter_osmosis = CounterContract::new(osmosis.clone());
    let vfs_osmosis = VFSContract::new(osmosis.clone());
    let adodb_osmosis = ADODBContract::new(osmosis.clone());
    let splitter_osmosis = SplitterContract::new(osmosis.clone());

    kernel_juno.upload().unwrap();
    vfs_juno.upload().unwrap();
    kernel_osmosis.upload().unwrap();
    counter_osmosis.upload().unwrap();
    vfs_osmosis.upload().unwrap();
    adodb_osmosis.upload().unwrap();
    splitter_osmosis.upload().unwrap();

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

    kernel_juno
        .execute(
            &ExecuteMsg::UpsertKeyAddress {
                key: "vfs".to_string(),
                value: vfs_juno.address().unwrap().into_string(),
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
                ics20_channel_id: Some(channel.0.channel.unwrap().to_string()),
                direct_channel_id: Some(juno_channel.to_string()),
                chain: "juno".to_string(),
                kernel_address: kernel_juno.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();

    let kernel_juno_send_request = kernel_juno
        .execute(
            &ExecuteMsg::Send {
                message: AMPMsg {
                    recipient: AndrAddr::from_string(format!(
                        "ibc://osmosis/{}",
                        kernel_osmosis.address().unwrap()
                    )),
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

    // For testing a successful outcome of the first packet sent out in the tx, you can use:
    if let IbcPacketOutcome::Success { .. } = &packet_lifetime.packets[0].outcome {
        // Packet has been successfully acknowledged and decoded, the transaction has gone through correctly
    } else {
        panic!("packet timed out");
        // There was a decode error or the packet timed out
        // Else the packet timed-out, you may have a relayer error or something is wrong in your application
    };
}
#[test]
fn test_kernel_ibc_funds_and_execute_msg() {
    // Here `juno-1` is the chain-id and `juno` is the address prefix for this chain
    let sender = Addr::unchecked("sender_for_all_chains").into_string();

    let interchain = MockInterchainEnv::new(vec![("juno", &sender), ("osmosis", &sender)]);

    let juno = interchain.get_chain("juno").unwrap();
    let osmosis = interchain.get_chain("osmosis").unwrap();

    juno.set_balance(sender.clone(), vec![Coin::new(100000000000000, "juno")])
        .unwrap();

    let kernel_juno = KernelContract::new(juno.clone());
    let vfs_juno = VFSContract::new(juno.clone());
    let kernel_osmosis = KernelContract::new(osmosis.clone());
    let counter_osmosis = CounterContract::new(osmosis.clone());
    let vfs_osmosis = VFSContract::new(osmosis.clone());
    let economics_osmosis = EconomicsContract::new(osmosis.clone());
    let adodb_osmosis = ADODBContract::new(osmosis.clone());
    let splitter_osmosis = SplitterContract::new(osmosis.clone());

    kernel_juno.upload().unwrap();
    vfs_juno.upload().unwrap();
    kernel_osmosis.upload().unwrap();
    counter_osmosis.upload().unwrap();
    vfs_osmosis.upload().unwrap();
    adodb_osmosis.upload().unwrap();
    splitter_osmosis.upload().unwrap();
    economics_osmosis.upload().unwrap();

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

    economics_osmosis
        .instantiate(
            &os::economics::InstantiateMsg {
                kernel_address: kernel_osmosis.address().unwrap().into_string(),
                owner: None,
            },
            None,
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
                code_id: splitter_osmosis.code_id().unwrap(),
                ado_type: "splitter".to_string(),
                action_fees: None,
                version: "1.0.0".to_string(),
                publisher: None,
            },
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

    kernel_juno
        .execute(
            &ExecuteMsg::UpsertKeyAddress {
                key: "vfs".to_string(),
                value: vfs_juno.address().unwrap().into_string(),
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

    kernel_osmosis
        .execute(
            &ExecuteMsg::UpsertKeyAddress {
                key: "economics".to_string(),
                value: economics_osmosis.address().unwrap().into_string(),
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
                ics20_channel_id: Some(channel.0.channel.unwrap().to_string()),
                direct_channel_id: Some(juno_channel.to_string()),
                chain: "juno".to_string(),
                kernel_address: kernel_juno.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();

    let recipient = "osmo1qzskhrca90qy2yjjxqzq4yajy842x7c50xq33d";

    // This section covers the actions that take place after a successful ack from the ICS20 transfer is received
    // Let's instantiate a splitter
    splitter_osmosis
        .instantiate(
            &SplitterInstantiateMsg {
                recipients: vec![AddressPercent {
                    recipient: Recipient {
                        address: AndrAddr::from_string(recipient),
                        msg: None,
                        ibc_recovery_address: None,
                    },
                    percent: Decimal::one(),
                }],
                lock_time: None,
                kernel_address: kernel_osmosis.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    let kernel_juno_send_request = kernel_juno
        .execute(
            &ExecuteMsg::Send {
                message: AMPMsg {
                    recipient: AndrAddr::from_string(format!(
                        "ibc://osmosis/{}",
                        splitter_osmosis.address().unwrap()
                    )),
                    message: to_json_binary(&SplitterExecuteMsg::Send {}).unwrap(),
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

    // For testing a successful outcome of the first packet sent out in the tx, you can use:
    if let IbcPacketOutcome::Success { .. } = &packet_lifetime.packets[0].outcome {
        // Register trigger address
        kernel_juno
            .execute(
                &ExecuteMsg::UpsertKeyAddress {
                    key: "trigger_key".to_string(),
                    value: sender,
                },
                None,
            )
            .unwrap();

        // Construct an Execute msg from the kernel on juno inteded for the splitter on osmosis
        let kernel_juno_splitter_request = kernel_juno
            .execute(
                &ExecuteMsg::TriggerRelay {
                    packet_sequence: "1".to_string(),
                    pack_ack_msg: IbcPacketAckMsg::new(
                        IbcAcknowledgement::new(
                            to_json_binary(
                                &AcknowledgementMsg::<SendMessageWithFundsResponse>::Ok(
                                    SendMessageWithFundsResponse {},
                                ),
                            )
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
        let balances = osmosis
            .query_all_balances(kernel_osmosis.address().unwrap())
            .unwrap();
        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].denom, "ibc/channel-0/juno");
        assert_eq!(balances[0].amount.u128(), 100);

        let packet_lifetime = interchain
            .await_packets("juno", kernel_juno_splitter_request)
            .unwrap();

        // For testing a successful outcome of the first packet sent out in the tx, you can use:
        if let IbcPacketOutcome::Success { .. } = &packet_lifetime.packets[0].outcome {
            // Packet has been successfully acknowledged and decoded, the transaction has gone through correctly
        } else {
            panic!("packet timed out");
            // There was a decode error or the packet timed out
            // Else the packet timed-out, you may have a relayer error or something is wrong in your application
        };

        // Packet has been successfully acknowledged and decoded, the transaction has gone through correctly
    } else {
        panic!("packet timed out");
        // There was a decode error or the packet timed out
        // Else the packet timed-out, you may have a relayer error or something is wrong in your application
    };
}

// Unhappy paths //
#[test]
fn test_kernel_ibc_funds_only_unhappy() {
    // Here `juno-1` is the chain-id and `juno` is the address prefix for this chain
    let sender = Addr::unchecked("sender_for_all_chains").into_string();

    let interchain = MockInterchainEnv::new(vec![("juno", &sender), ("osmosis", &sender)]);

    let juno = interchain.get_chain("juno").unwrap();
    let osmosis = interchain.get_chain("osmosis").unwrap();

    juno.set_balance(sender.clone(), vec![Coin::new(100000000000000, "juno")])
        .unwrap();

    let kernel_juno = KernelContract::new(juno.clone());
    let vfs_juno = VFSContract::new(juno.clone());
    let kernel_osmosis = KernelContract::new(osmosis.clone());
    let counter_osmosis = CounterContract::new(osmosis.clone());
    let vfs_osmosis = VFSContract::new(osmosis.clone());
    let adodb_osmosis = ADODBContract::new(osmosis.clone());
    let splitter_osmosis = SplitterContract::new(osmosis.clone());

    kernel_juno.upload().unwrap();
    vfs_juno.upload().unwrap();
    kernel_osmosis.upload().unwrap();
    counter_osmosis.upload().unwrap();
    vfs_osmosis.upload().unwrap();
    adodb_osmosis.upload().unwrap();
    splitter_osmosis.upload().unwrap();

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

    kernel_juno
        .execute(
            &ExecuteMsg::UpsertKeyAddress {
                key: "vfs".to_string(),
                value: vfs_juno.address().unwrap().into_string(),
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
                ics20_channel_id: Some(channel.0.channel.unwrap().to_string()),
                direct_channel_id: Some(juno_channel.to_string()),
                chain: "juno".to_string(),
                kernel_address: kernel_juno.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();
    let balances = juno.query_all_balances(sender.clone()).unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].denom, "juno");
    println!("sender balance before transfer: {}", balances[0].amount);

    let kernel_juno_send_request = kernel_juno
        .execute(
            &ExecuteMsg::Send {
                message: AMPMsg {
                    recipient: AndrAddr::from_string(format!(
                        "ibc://osmosis/{}",
                        kernel_osmosis.address().unwrap()
                    )),
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

    osmosis.wait_seconds(604_810).unwrap();

    let packet_lifetime = interchain
        .await_packets("juno", kernel_juno_send_request)
        .unwrap();

    // For testing a successful outcome of the first packet sent out in the tx, you can use:
    if let IbcPacketOutcome::Success { .. } = &packet_lifetime.packets[0].outcome {
        // Packet has been successfully acknowledged and decoded, the transaction has gone through correctly
    } else {
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
        let kernel_juno_splitter_request = kernel_juno
            .execute(
                &ExecuteMsg::TriggerRelay {
                    packet_sequence: "1".to_string(),
                    pack_ack_msg: IbcPacketAckMsg::new(
                        IbcAcknowledgement::new(
                            to_json_binary(
                                &AcknowledgementMsg::<SendMessageWithFundsResponse>::Error(
                                    "error".to_string(),
                                ),
                            )
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
        let _packet_lifetime = interchain
            .await_packets("juno", kernel_juno_splitter_request)
            .unwrap();

        let balances = juno.query_all_balances(sender).unwrap();
        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].denom, "juno");
        // Original amount
        assert_eq!(balances[0].amount, Uint128::new(100000000000000));

        // Make sure kernel has no funds
        let balances = juno
            .query_all_balances(kernel_juno.address().unwrap())
            .unwrap();
        assert_eq!(balances.len(), 0);
        // There was a decode error or the packet timed out
        // Else the packet timed-out, you may have a relayer error or something is wrong in your application
    };
}

// #[test]
// fn test_kernel_ibc_funds_and_execute_msg_unhappy() {
//     // Here `juno-1` is the chain-id and `juno` is the address prefix for this chain
//     let sender = Addr::unchecked("sender_for_all_chains").into_string();

//     let interchain = MockInterchainEnv::new(vec![("juno", &sender), ("osmosis", &sender)]);

//     let juno = interchain.get_chain("juno").unwrap();
//     let osmosis = interchain.get_chain("osmosis").unwrap();

//     juno.set_balance(sender.clone(), vec![Coin::new(100000000000000, "juno")])
//         .unwrap();

//     let kernel_juno = KernelContract::new(juno.clone());
//     let vfs_juno = VFSContract::new(juno.clone());
//     let kernel_osmosis = KernelContract::new(osmosis.clone());
//     let counter_osmosis = CounterContract::new(osmosis.clone());
//     let vfs_osmosis = VFSContract::new(osmosis.clone());
//     let economics_osmosis = EconomicsContract::new(osmosis.clone());
//     let adodb_osmosis = ADODBContract::new(osmosis.clone());
//     let splitter_osmosis = SplitterContract::new(osmosis.clone());

//     kernel_juno.upload().unwrap();
//     vfs_juno.upload().unwrap();
//     kernel_osmosis.upload().unwrap();
//     counter_osmosis.upload().unwrap();
//     vfs_osmosis.upload().unwrap();
//     adodb_osmosis.upload().unwrap();
//     splitter_osmosis.upload().unwrap();
//     economics_osmosis.upload().unwrap();

//     let init_msg_juno = &InstantiateMsg {
//         owner: None,
//         chain_name: "juno".to_string(),
//     };
//     let init_msg_osmosis = &InstantiateMsg {
//         owner: None,
//         chain_name: "osmosis".to_string(),
//     };

//     kernel_juno.instantiate(init_msg_juno, None, None).unwrap();
//     kernel_osmosis
//         .instantiate(init_msg_osmosis, None, None)
//         .unwrap();

//     // Set up channel from juno to osmosis
//     let channel_receipt = interchain
//         .create_contract_channel(&kernel_juno, &kernel_osmosis, "andr-kernel-1", None)
//         .unwrap();

//     // After channel creation is complete, we get the channel id, which is necessary for ICA remote execution
//     let juno_channel = channel_receipt
//         .interchain_channel
//         .get_chain("juno")
//         .unwrap()
//         .channel
//         .unwrap();

//     // Set up channel from juno to osmosis for ICS20 transfers
//     let channel_receipt = interchain
//         .create_channel(
//             "juno",
//             "osmosis",
//             &PortId::transfer(),
//             &PortId::transfer(),
//             "ics20-1",
//             None,
//         )
//         .unwrap();

//     let channel = channel_receipt
//         .interchain_channel
//         .get_ordered_ports_from("juno")
//         .unwrap();

//     // After channel creation is complete, we get the channel id, which is necessary for ICA remote execution
//     let _juno_channel_ics20 = channel_receipt
//         .interchain_channel
//         .get_chain("juno")
//         .unwrap()
//         .channel
//         .unwrap();

//     vfs_juno
//         .instantiate(
//             &os::vfs::InstantiateMsg {
//                 kernel_address: kernel_juno.address().unwrap().into_string(),
//                 owner: None,
//             },
//             None,
//             None,
//         )
//         .unwrap();

//     vfs_osmosis
//         .instantiate(
//             &os::vfs::InstantiateMsg {
//                 kernel_address: kernel_osmosis.address().unwrap().into_string(),
//                 owner: None,
//             },
//             None,
//             None,
//         )
//         .unwrap();

//     economics_osmosis
//         .instantiate(
//             &os::economics::InstantiateMsg {
//                 kernel_address: kernel_osmosis.address().unwrap().into_string(),
//                 owner: None,
//             },
//             None,
//             None,
//         )
//         .unwrap();

//     adodb_osmosis
//         .instantiate(
//             &os::adodb::InstantiateMsg {
//                 kernel_address: kernel_osmosis.address().unwrap().into_string(),
//                 owner: None,
//             },
//             None,
//             None,
//         )
//         .unwrap();

//     adodb_osmosis
//         .execute(
//             &os::adodb::ExecuteMsg::Publish {
//                 code_id: splitter_osmosis.code_id().unwrap(),
//                 ado_type: "splitter".to_string(),
//                 action_fees: None,
//                 version: "1.0.0".to_string(),
//                 publisher: None,
//             },
//             None,
//         )
//         .unwrap();

//     adodb_osmosis
//         .execute(
//             &os::adodb::ExecuteMsg::Publish {
//                 code_id: 2,
//                 ado_type: "counter".to_string(),
//                 action_fees: None,
//                 version: "1.0.2".to_string(),
//                 publisher: None,
//             },
//             None,
//         )
//         .unwrap();

//     kernel_juno
//         .execute(
//             &ExecuteMsg::UpsertKeyAddress {
//                 key: "vfs".to_string(),
//                 value: vfs_juno.address().unwrap().into_string(),
//             },
//             None,
//         )
//         .unwrap();

//     kernel_osmosis
//         .execute(
//             &ExecuteMsg::UpsertKeyAddress {
//                 key: "vfs".to_string(),
//                 value: vfs_osmosis.address().unwrap().into_string(),
//             },
//             None,
//         )
//         .unwrap();

//     kernel_osmosis
//         .execute(
//             &ExecuteMsg::UpsertKeyAddress {
//                 key: "adodb".to_string(),
//                 value: adodb_osmosis.address().unwrap().into_string(),
//             },
//             None,
//         )
//         .unwrap();

//     kernel_osmosis
//         .execute(
//             &ExecuteMsg::UpsertKeyAddress {
//                 key: "economics".to_string(),
//                 value: economics_osmosis.address().unwrap().into_string(),
//             },
//             None,
//         )
//         .unwrap();

//     kernel_juno
//         .execute(
//             &ExecuteMsg::AssignChannels {
//                 ics20_channel_id: Some(channel.clone().0.channel.unwrap().to_string()),
//                 direct_channel_id: Some(juno_channel.to_string()),
//                 chain: "osmosis".to_string(),
//                 kernel_address: kernel_osmosis.address().unwrap().into_string(),
//             },
//             None,
//         )
//         .unwrap();

//     kernel_osmosis
//         .execute(
//             &ExecuteMsg::AssignChannels {
//                 ics20_channel_id: Some(channel.0.channel.unwrap().to_string()),
//                 direct_channel_id: Some(juno_channel.to_string()),
//                 chain: "juno".to_string(),
//                 kernel_address: kernel_juno.address().unwrap().into_string(),
//             },
//             None,
//         )
//         .unwrap();

//     let recipient = "osmo1qzskhrca90qy2yjjxqzq4yajy842x7c50xq33d";

//     // This section covers the actions that take place after a successful ack from the ICS20 transfer is received
//     // Let's instantiate a splitter
//     splitter_osmosis
//         .instantiate(
//             &SplitterInstantiateMsg {
//                 recipients: vec![AddressPercent {
//                     recipient: Recipient {
//                         address: AndrAddr::from_string(recipient),
//                         msg: None,
//                         ibc_recovery_address: None,
//                     },
//                     percent: Decimal::one(),
//                 }],
//                 lock_time: None,
//                 kernel_address: kernel_osmosis.address().unwrap().into_string(),
//                 owner: None,
//             },
//             None,
//             None,
//         )
//         .unwrap();

//     let kernel_juno_send_request = kernel_juno
//         .execute(
//             &ExecuteMsg::Send {
//                 message: AMPMsg {
//                     recipient: AndrAddr::from_string(format!(
//                         "ibc://osmosis/{}",
//                         splitter_osmosis.address().unwrap()
//                     )),
//                     // Send invalid message to the splitter
//                     message: to_json_binary(&Binary::default()).unwrap(),
//                     funds: vec![Coin {
//                         denom: "juno".to_string(),
//                         amount: Uint128::new(100),
//                     }],
//                     config: AMPMsgConfig {
//                         reply_on: cosmwasm_std::ReplyOn::Always,
//                         exit_at_error: false,
//                         gas_limit: None,
//                         direct: true,
//                         ibc_config: None,
//                     },
//                 },
//             },
//             Some(&[Coin {
//                 denom: "juno".to_string(),
//                 amount: Uint128::new(100),
//             }]),
//         )
//         .unwrap();

//     let packet_lifetime = interchain
//         .await_packets("juno", kernel_juno_send_request)
//         .unwrap();

//     // For testing a successful outcome of the first packet sent out in the tx, you can use:
//     if let IbcPacketOutcome::Success { .. } = &packet_lifetime.packets[0].outcome {
//         // Register trigger address
//         kernel_juno
//             .execute(
//                 &ExecuteMsg::UpsertKeyAddress {
//                     key: "trigger_key".to_string(),
//                     value: sender.clone(),
//                 },
//                 None,
//             )
//             .unwrap();

//         // Construct an Execute msg from the kernel on juno inteded for the splitter on osmosis
//         let kernel_juno_splitter_request = kernel_juno
//             .execute(
//                 &ExecuteMsg::TriggerRelay {
//                     packet_sequence: "1".to_string(),
//                     pack_ack_msg: IbcPacketAckMsg::new(
//                         IbcAcknowledgement::new(
//                             to_json_binary(
//                                 &AcknowledgementMsg::<SendMessageWithFundsResponse>::Error(
//                                     "error".to_string(),
//                                 ),
//                             )
//                             .unwrap(),
//                         ),
//                         IbcPacket::new(
//                             Binary::default(),
//                             IbcEndpoint {
//                                 port_id: "port_id".to_string(),
//                                 channel_id: "channel_id".to_string(),
//                             },
//                             IbcEndpoint {
//                                 port_id: "port_id".to_string(),
//                                 channel_id: "channel_id".to_string(),
//                             },
//                             1,
//                             IbcTimeout::with_timestamp(Timestamp::from_seconds(1)),
//                         ),
//                         Addr::unchecked("relayer"),
//                     ),
//                 },
//                 None,
//             )
//             .unwrap();
//         let balances = juno.query_all_balances(sender).unwrap();
//         assert_eq!(balances.len(), 1);
//         assert_eq!(balances[0].denom, "juno");
//         // Starting amount
//         assert_eq!(balances[0].amount.u128(), 100000000000000);

//         let packet_lifetime = interchain
//             .await_packets("juno", kernel_juno_splitter_request)
//             .unwrap();

//         // For testing a successful outcome of the first packet sent out in the tx, you can use:
//         if let IbcPacketOutcome::Success { .. } = &packet_lifetime.packets[0].outcome {
//             // Packet has been successfully acknowledged and decoded, the transaction has gone through correctly
//         } else {
//             panic!("packet timed out");
//             // There was a decode error or the packet timed out
//             // Else the packet timed-out, you may have a relayer error or something is wrong in your application
//         };

//         // Packet has been successfully acknowledged and decoded, the transaction has gone through correctly
//     } else {
//         panic!("packet timed out");
//         // There was a decode error or the packet timed out
//         // Else the packet timed-out, you may have a relayer error or something is wrong in your application
//     };
// }
