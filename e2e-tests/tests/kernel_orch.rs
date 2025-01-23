use andromeda_adodb::ADODBContract;
use andromeda_auction::{mock::mock_start_auction, AuctionContract};
use andromeda_counter::CounterContract;
use andromeda_economics::EconomicsContract;
use andromeda_finance::splitter::{
    AddressPercent, ExecuteMsg as SplitterExecuteMsg, InstantiateMsg as SplitterInstantiateMsg,
};
use andromeda_math::counter::{
    CounterRestriction, ExecuteMsg as CounterExecuteMsg, GetCurrentAmountResponse,
    InstantiateMsg as CounterInstantiateMsg, State,
};

use andromeda_cw721::CW721Contract;
use andromeda_kernel::KernelContract;
use andromeda_non_fungible_tokens::cw721::TokenExtension;
use andromeda_splitter::SplitterContract;
use andromeda_std::{
    ado_base::rates::{LocalRate, LocalRateType, LocalRateValue, PercentRate, Rate, RatesMessage},
    amp::{
        messages::{AMPMsg, AMPMsgConfig},
        AndrAddr, Recipient,
    },
    common::{denom::Asset, expiration::Expiry, Milliseconds},
    os::{
        self,
        kernel::{ExecuteMsg, InstantiateMsg},
    },
};
use andromeda_vfs::VFSContract;
use cosmwasm_std::{coin, to_json_binary, Addr, Binary, Decimal, StdAck, Uint128};
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
    let economics_osmosis = EconomicsContract::new(osmosis.clone());

    kernel_juno.upload().unwrap();
    vfs_juno.upload().unwrap();

    kernel_osmosis.upload().unwrap();
    counter_osmosis.upload().unwrap();
    vfs_osmosis.upload().unwrap();
    adodb_osmosis.upload().unwrap();
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
        .query(&andromeda_math::counter::QueryMsg::GetCurrentAmount {})
        .unwrap();
    assert_eq!(current_count.current_amount, 1);
}

#[test]
fn test_kernel_ibc_execute_only_with_username() {
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
    let economics_osmosis = EconomicsContract::new(osmosis.clone());

    kernel_juno.upload().unwrap();
    vfs_juno.upload().unwrap();
    kernel_osmosis.upload().unwrap();
    counter_osmosis.upload().unwrap();
    vfs_osmosis.upload().unwrap();
    adodb_osmosis.upload().unwrap();
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

    // Register username for sender
    vfs_juno
        .call_as(&kernel_juno.address().unwrap())
        .execute(
            &os::vfs::ExecuteMsg::RegisterUser {
                username: "az".to_string(),
                address: Some(kernel_juno.address().unwrap()),
            },
            None,
        )
        .unwrap();

    vfs_osmosis
        .call_as(&kernel_osmosis.address().unwrap())
        .execute(
            &os::vfs::ExecuteMsg::RegisterUser {
                username: "az".to_string(),
                address: Some(kernel_osmosis.address().unwrap()),
            },
            None,
        )
        .unwrap();

    let kernel_juno_send_request = kernel_juno
        .call_as(&kernel_juno.address().unwrap())
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
    if let IbcPacketOutcome::Success { receive_tx, .. } = &packet_lifetime.packets[0].outcome {
        let username = receive_tx
            .event_attr_value("recv_packet", "packet_data")
            .unwrap();
        assert!(username.contains("az"));
        // println!("success_packets: {:?}", success_packets);
        // Packet has been successfully acknowledged and decoded, the transaction has gone through correctly
    } else {
        panic!("packet timed out");
        // There was a decode error or the packet timed out
        // Else the packet timed-out, you may have a relayer error or something is wrong in your application
    };

    let current_count: GetCurrentAmountResponse = counter_osmosis
        .query(&andromeda_math::counter::QueryMsg::GetCurrentAmount {})
        .unwrap();
    assert_eq!(current_count.current_amount, 1);
}

#[test]
fn test_kernel_ibc_execute_only_multi_hop() {
    // Here `juno-1` is the chain-id and `juno` is the address prefix for this chain
    let sender = Addr::unchecked("sender_for_all_chains").into_string();

    let interchain = MockInterchainEnv::new(vec![
        ("juno", &sender),
        ("osmosis", &sender),
        ("andromeda", &sender),
    ]);

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
    let andromeda = interchain.get_chain("andromeda").unwrap();
    let kernel_andromeda = KernelContract::new(andromeda.clone());
    let counter_andromeda = CounterContract::new(andromeda.clone());
    let vfs_andromeda = VFSContract::new(andromeda.clone());
    let adodb_andromeda = ADODBContract::new(andromeda.clone());
    let economics_andromeda = EconomicsContract::new(andromeda.clone());

    kernel_andromeda.upload().unwrap();
    counter_andromeda.upload().unwrap();
    vfs_andromeda.upload().unwrap();
    adodb_andromeda.upload().unwrap();
    economics_andromeda.upload().unwrap();

    let init_msg_andromeda = &InstantiateMsg {
        owner: None,
        chain_name: "andromeda".to_string(),
    };

    kernel_andromeda
        .instantiate(init_msg_andromeda, None, None)
        .unwrap();

    vfs_andromeda
        .instantiate(
            &os::vfs::InstantiateMsg {
                kernel_address: kernel_andromeda.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    adodb_andromeda
        .instantiate(
            &os::adodb::InstantiateMsg {
                kernel_address: kernel_andromeda.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    economics_andromeda
        .instantiate(
            &os::economics::InstantiateMsg {
                kernel_address: kernel_andromeda.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    adodb_andromeda
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

    kernel_andromeda
        .execute(
            &ExecuteMsg::UpsertKeyAddress {
                key: "adodb".to_string(),
                value: adodb_andromeda.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();

    kernel_andromeda
        .execute(
            &ExecuteMsg::UpsertKeyAddress {
                key: "economics".to_string(),
                value: economics_andromeda.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();

    // Set up channel from osmosis to andromeda
    let channel_receipt_2 = interchain
        .create_contract_channel(&kernel_osmosis, &kernel_andromeda, "andr-kernel-1", None)
        .unwrap();

    // After channel creation is complete, we get the channel id, which is necessary for ICA remote execution
    let osmosis_channel = channel_receipt_2
        .interchain_channel
        .get_chain("osmosis")
        .unwrap()
        .channel
        .unwrap();

    kernel_osmosis
        .execute(
            &ExecuteMsg::AssignChannels {
                ics20_channel_id: None,
                direct_channel_id: Some(osmosis_channel.to_string()),
                chain: "andromeda".to_string(),
                kernel_address: kernel_andromeda.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();

    kernel_andromeda
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

    counter_andromeda
        .instantiate(
            &CounterInstantiateMsg {
                restriction: CounterRestriction::Public,
                initial_state: State {
                    initial_amount: None,
                    increase_amount: Some(1),
                    decrease_amount: None,
                },
                kernel_address: kernel_andromeda.address().unwrap().into_string(),
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
                        "ibc://osmosis/ibc://andromeda/{}",
                        counter_andromeda.address().unwrap()
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

    interchain
        .await_and_check_packets("juno", kernel_juno_send_request.clone())
        .unwrap();

    // // For testing a successful outcome of the first packet sent out in the tx, you can use:
    // if let IbcPacketOutcome::Success { .. } = &packet_lifetime.packets[0].outcome {
    //     // Packet has been successfully acknowledged and decoded, the transaction has gone through correctly
    // } else {
    //     panic!("packet timed out");
    //     // There was a decode error or the packet timed out
    //     // Else the packet timed-out, you may have a relayer error or something is wrong in your application
    // };

    // Send a message to the counter on andromeda

    let current_count: GetCurrentAmountResponse = counter_andromeda
        .query(&andromeda_math::counter::QueryMsg::GetCurrentAmount {})
        .unwrap();
    assert_eq!(current_count.current_amount, 1);
}

#[test]
fn test_kernel_ibc_funds_only() {
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
    let mut auction_juno = AuctionContract::new(juno.clone());
    let cw721_juno = CW721Contract::new(juno.clone());
    let kernel_osmosis = KernelContract::new(osmosis.clone());
    let counter_osmosis = CounterContract::new(osmosis.clone());
    let vfs_osmosis = VFSContract::new(osmosis.clone());
    let adodb_osmosis = ADODBContract::new(osmosis.clone());
    let splitter_osmosis = SplitterContract::new(osmosis.clone());

    kernel_juno.upload().unwrap();
    vfs_juno.upload().unwrap();
    adodb_juno.upload().unwrap();
    economics_juno.upload().unwrap();
    auction_juno.upload().unwrap();
    cw721_juno.upload().unwrap();

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

    let kernel_juno_send_request = kernel_juno
        .execute(
            &ExecuteMsg::Send {
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
        assert_eq!(balances[0].amount.u128(), 100);
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

    // Set up cross chain rates recipient
    auction_juno
        .instantiate(
            &andromeda_non_fungible_tokens::auction::InstantiateMsg {
                authorized_token_addresses: None,
                authorized_cw20_addresses: None,
                kernel_address: kernel_juno.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    cw721_juno
        .instantiate(
            &andromeda_non_fungible_tokens::cw721::InstantiateMsg {
                name: "test tokens".to_string(),
                symbol: "TT".to_string(),
                minter: AndrAddr::from_string(sender.clone()),
                kernel_address: kernel_juno.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    auction_juno
        .execute(
            &andromeda_non_fungible_tokens::auction::ExecuteMsg::Rates(RatesMessage::SetRate {
                action: "Claim".to_string(),
                rate: Rate::Local(LocalRate {
                    rate_type: LocalRateType::Deductive,
                    recipient: Recipient::new(
                        AndrAddr::from_string(format!("ibc://osmosis/{}", recipient)),
                        None,
                    ),
                    value: LocalRateValue::Percent(PercentRate {
                        percent: Decimal::percent(50),
                    }),
                    description: None,
                }),
            }),
            None,
        )
        .unwrap();

    cw721_juno
        .execute(
            &andromeda_non_fungible_tokens::cw721::ExecuteMsg::Mint {
                token_id: "1".to_string(),
                owner: sender.clone(),
                token_uri: None,
                extension: TokenExtension::default(),
            },
            None,
        )
        .unwrap();

    let start_time = Milliseconds::from_nanos(juno.block_info().unwrap().time.nanos());
    let receive_msg = mock_start_auction(
        None,
        Expiry::AtTime(start_time.plus_milliseconds(Milliseconds(10000))),
        None,
        Asset::NativeToken("juno".to_string()),
        None,
        None,
        None,
        None,
    );
    cw721_juno
        .execute(
            &andromeda_non_fungible_tokens::cw721::ExecuteMsg::SendNft {
                contract: AndrAddr::from_string(auction_juno.address().unwrap()),
                token_id: "1".to_string(),
                msg: to_json_binary(&receive_msg).unwrap(),
            },
            None,
        )
        .unwrap();
    juno.wait_seconds(1).unwrap();

    auction_juno.set_sender(&Addr::unchecked(buyer.clone()));
    auction_juno
        .execute(
            &andromeda_non_fungible_tokens::auction::ExecuteMsg::PlaceBid {
                token_id: "1".to_string(),
                token_address: cw721_juno.address().unwrap().into_string(),
            },
            Some(&[coin(50, "juno")]),
        )
        .unwrap();
    juno.next_block().unwrap();
    juno.next_block().unwrap();

    // Claim
    let claim_request = auction_juno
        .execute(
            &andromeda_non_fungible_tokens::auction::ExecuteMsg::Claim {
                token_id: "1".to_string(),
                token_address: cw721_juno.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();
    let packet_lifetime = interchain.await_packets("juno", claim_request).unwrap();

    // For testing a successful outcome of the first packet sent out in the tx, you can use:
    if let IbcPacketOutcome::Success { .. } = &packet_lifetime.packets[0].outcome {
        // Packet has been successfully acknowledged and decoded, the transaction has gone through correctly

        // Check recipient balance after trigger execute msg
        let balances = osmosis
            .query_all_balances(kernel_osmosis.address().unwrap())
            .unwrap();
        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].denom, ibc_denom);
        assert_eq!(balances[0].amount.u128(), 25);
    } else {
        panic!("packet timed out");
        // There was a decode error or the packet timed out
        // Else the packet timed-out, you may have a relayer error or something is wrong in your application
    };

    // Construct an Execute msg from the kernel on juno inteded for the splitter on osmosis
    let kernel_juno_trigger_request = kernel_juno
        .execute(
            &ExecuteMsg::TriggerRelay {
                packet_sequence: 2,
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
        assert_eq!(balances[0].amount.u128(), 100 + 25);
    } else {
        panic!("packet timed out");
        // There was a decode error or the packet timed out
        // Else the packet timed-out, you may have a relayer error or something is wrong in your application
    };
}

#[test]
fn test_kernel_ibc_funds_only_multi_hop() {
    // Here `juno-1` is the chain-id and `juno` is the address prefix for this chain
    let sender = Addr::unchecked("sender_for_all_chains").into_string();

    let interchain = MockInterchainEnv::new(vec![
        ("juno", &sender),
        ("osmosis", &sender),
        ("andromeda", &sender),
    ]);

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
                ics20_channel_id: Some(channel.clone().0.channel.unwrap().to_string()),
                direct_channel_id: Some(juno_channel.to_string()),
                chain: "juno".to_string(),
                kernel_address: kernel_juno.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();
    // Connecting Andromeda part
    let andromeda = interchain.get_chain("andromeda").unwrap();
    let kernel_andromeda = KernelContract::new(andromeda.clone());
    let counter_andromeda = CounterContract::new(andromeda.clone());
    let vfs_andromeda = VFSContract::new(andromeda.clone());
    let adodb_andromeda = ADODBContract::new(andromeda.clone());

    kernel_andromeda.upload().unwrap();
    counter_andromeda.upload().unwrap();
    vfs_andromeda.upload().unwrap();
    adodb_andromeda.upload().unwrap();

    let init_msg_andromeda = &InstantiateMsg {
        owner: None,
        chain_name: "andromeda".to_string(),
    };

    kernel_andromeda
        .instantiate(init_msg_andromeda, None, None)
        .unwrap();

    vfs_andromeda
        .instantiate(
            &os::vfs::InstantiateMsg {
                kernel_address: kernel_andromeda.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    adodb_andromeda
        .instantiate(
            &os::adodb::InstantiateMsg {
                kernel_address: kernel_andromeda.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    kernel_andromeda
        .execute(
            &ExecuteMsg::UpsertKeyAddress {
                key: "adodb".to_string(),
                value: adodb_andromeda.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();

    // Set up channel from osmosis to andromeda
    let channel_receipt_2 = interchain
        .create_contract_channel(&kernel_osmosis, &kernel_andromeda, "andr-kernel-1", None)
        .unwrap();

    // After channel creation is complete, we get the channel id, which is necessary for ICA remote execution
    let osmosis_channel = channel_receipt_2
        .interchain_channel
        .get_chain("osmosis")
        .unwrap()
        .channel
        .unwrap();

    kernel_osmosis
        .execute(
            &ExecuteMsg::AssignChannels {
                ics20_channel_id: None,
                direct_channel_id: Some(osmosis_channel.to_string()),
                chain: "andromeda".to_string(),
                kernel_address: kernel_andromeda.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();

    kernel_andromeda
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
                ics20_channel_id: Some(channel.0.channel.unwrap().to_string()),
                direct_channel_id: Some(juno_channel.to_string()),
                chain: "andromeda".to_string(),
                kernel_address: kernel_andromeda.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();

    let amp_msg = AMPMsg {
        recipient: AndrAddr::from_string(format!(
            "ibc://andromeda/{}",
            kernel_andromeda.address().unwrap()
        )),
        message: Binary::default(),
        funds: vec![Coin {
            denom: "ibc/channel-0/juno".to_string(),
            amount: Uint128::new(100),
        }],
        config: AMPMsgConfig {
            reply_on: cosmwasm_std::ReplyOn::Always,
            exit_at_error: false,
            gas_limit: None,
            direct: true,
            ibc_config: None,
        },
    };
    let kernel_juno_send_request = kernel_juno
        .execute(
            &ExecuteMsg::Send {
                message: AMPMsg {
                    recipient: AndrAddr::from_string(format!(
                        "ibc://osmosis/{}",
                        kernel_osmosis.address().unwrap()
                    )),
                    message: to_json_binary(&ExecuteMsg::Send { message: amp_msg }).unwrap(),
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

    let balances = osmosis
        .query_all_balances(kernel_andromeda.address().unwrap())
        .unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].denom, "ibc/channel-0/juno");
    assert_eq!(balances[0].amount.u128(), 100);
}

#[test]
fn test_kernel_ibc_funds_and_execute_msg() {
    // Here `juno-1` is the chain-id and `juno` is the address prefix for this chain
    let sender = Addr::unchecked("sender_for_all_chains").into_string();

    let interchain = MockInterchainEnv::new(vec![
        ("juno", &sender),
        ("osmosis", &sender),
        ("cosmoshub", &sender),
    ]);

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

    // Set up channel from juno to cosmoshub for ICS20 transfers so that channel-0 is used on osmosis
    // Later when we create channel with juno, channel-1 will be used on juno
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
                ics20_channel_id: Some(channel.0.channel.clone().unwrap().to_string()),
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
                default_recipient: None,
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
                    message: to_json_binary(&SplitterExecuteMsg::Send { config: None }).unwrap(),
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
        let ibc_denom = format!("ibc/{}/{}", channel.1.channel.unwrap().as_str(), "juno");
        // Check kernel balance before trigger execute msg
        let balances = osmosis
            .query_all_balances(kernel_osmosis.address().unwrap())
            .unwrap();
        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].denom, ibc_denom);
        assert_eq!(balances[0].amount.u128(), 100);

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
                    packet_sequence: 1,
                    channel_id: channel.0.channel.clone().unwrap().to_string(),
                    packet_ack: to_json_binary(&StdAck::Success(Binary::default())).unwrap(),
                },
                None,
            )
            .unwrap();

        let packet_lifetime = interchain
            .await_packets("juno", kernel_juno_splitter_request)
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
                ics20_channel_id: Some(channel.0.channel.clone().unwrap().to_string()),
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
                    packet_sequence: 1,
                    channel_id: channel.0.channel.clone().unwrap().to_string(),
                    packet_ack: to_json_binary(&StdAck::Error("error".to_string())).unwrap(),
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

#[test]
fn test_kernel_ibc_funds_and_execute_msg_unhappy() {
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
                ics20_channel_id: Some(channel.0.channel.clone().unwrap().to_string()),
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
                default_recipient: None,
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
                    // Send invalid message to the splitter. It's invalid because we'll be attaching funds to it and the msg rejects funds
                    message: to_json_binary(&SplitterExecuteMsg::UpdateLock {
                        lock_time: andromeda_std::common::expiration::Expiry::AtTime(
                            Milliseconds::zero(),
                        ),
                    })
                    .unwrap(),
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
    // Make sure the sender's balance decreased by 100
    let balances = juno.query_all_balances(sender.clone()).unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].denom, "juno");
    // Original amount
    assert_eq!(balances[0].amount, Uint128::new(100000000000000 - 100));

    // For testing a successful outcome of the first packet sent out in the tx, you can use:
    if let IbcPacketOutcome::Success { .. } = &packet_lifetime.packets[0].outcome {
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
        let kernel_juno_splitter_request = kernel_juno
            .execute(
                &ExecuteMsg::TriggerRelay {
                    packet_sequence: 1,
                    channel_id: channel.0.channel.clone().unwrap().to_string(),
                    packet_ack: to_json_binary(&StdAck::Success(Binary::default())).unwrap(),
                },
                None,
            )
            .unwrap();
        // We call UpadeLock, a Msg that doesn't accept funds. So it will error and should trigger a refund from the destination chain
        interchain
            .await_and_check_packets("juno", kernel_juno_splitter_request.clone())
            .unwrap();

        // Make sure kernel has no funds
        let balances = juno
            .query_all_balances(kernel_juno.address().unwrap())
            .unwrap();
        assert_eq!(balances.len(), 0);

        let balances = juno.query_all_balances(sender).unwrap();
        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].denom, "juno");
        // Original amount
        assert_eq!(balances[0].amount, Uint128::new(100000000000000));

        // // For testing a successful outcome of the first packet sent out in the tx, you can use:
        // if let IbcPacketOutcome::Success { .. } = &packet_lifetime.packets[0].outcome {
        //     // Packet has been successfully acknowledged and decoded, the transaction has gone through correctly
        // } else {
        //     panic!("packet timed out");
        //     // There was a decode error or the packet timed out
        //     // Else the packet timed-out, you may have a relayer error or something is wrong in your application
        // };

        // Packet has been successfully acknowledged and decoded, the transaction has gone through correctly
    } else {
        panic!("packet timed out");
        // There was a decode error or the packet timed out
        // Else the packet timed-out, you may have a relayer error or something is wrong in your application
    };
}
