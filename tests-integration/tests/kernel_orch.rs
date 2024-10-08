#![cfg(not(target_arch = "wasm32"))]
use andromeda_adodb::ADODBContract;
use andromeda_counter::CounterContract;
use andromeda_data_storage::counter::CounterRestriction;
use andromeda_data_storage::counter::ExecuteMsg as CounterExecuteMsg;
use andromeda_data_storage::counter::GetCurrentAmountResponse;
use andromeda_data_storage::counter::InstantiateMsg as CounterInstantiateMsg;
use andromeda_data_storage::counter::State;
use andromeda_kernel::KernelContract;
use andromeda_std::amp::messages::AMPMsg;
use andromeda_std::amp::messages::AMPMsgConfig;
use andromeda_std::amp::AndrAddr;
use andromeda_std::os;
use andromeda_std::os::kernel::ExecuteMsg;
use andromeda_std::os::kernel::InstantiateMsg;
use andromeda_vfs::VFSContract;
use cosmwasm_std::to_json_binary;
use cosmwasm_std::Addr;
use cw_orch::prelude::*;
use cw_orch_interchain::prelude::*;
use cw_orch_interchain::types::IbcPacketOutcome;
use cw_orch_interchain::InterchainEnv;

#[test]
fn test_kernel_ibc() {
    // Here `juno-1` is the chain-id and `juno` is the address prefix for this chain
    let sender = Addr::unchecked("sender_for_all_chains").into_string();

    let interchain = MockInterchainEnv::new(vec![("juno", &sender), ("osmosis", &sender)]);

    let juno = interchain.chain("juno").unwrap();
    let osmosis = interchain.chain("osmosis").unwrap();

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
        .wait_ibc("juno", kernel_juno_send_request)
        .unwrap();

    // For testing a successful outcome of the first packet sent out in the tx, you can use:
    if let IbcPacketOutcome::Success { ack, .. } = &packet_lifetime.packets[0].outcome {
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
