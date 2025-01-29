#![cfg(not(target_arch = "wasm32"))]

use std::vec;

use andromeda_splitter::SplitterContract;
use cosmwasm_std::{to_json_binary, Coin, Decimal, Uint128};
use cw_orch::mock::MockBase;
use andromeda_std::{
    amp::{messages::AMPMsg, recipient::{self, Recipient}, AndrAddr},
    os
};
use andromeda_kernel::ack::make_ack_success;
use andromeda_testing::{
    interchain::{ensure_packet_success, InterchainChain, DEFAULT_SENDER},
    InterchainTestEnv,
};
use cw_orch::prelude::*;
use cw_orch_interchain::prelude::*;
use andromeda_finance::splitter::{InstantiateMsg, AddressPercent};
use tests_integration::ado_deployer;

pub struct ChainMap<'a> {
    pub chains: Vec<(&'a InterchainChain, &'a InterchainChain)>,
}

ado_deployer!(
    deploy_splitter,
    InterchainChain,
    SplitterContract<MockBase>,
    &InstantiateMsg
);
#[test]
fn run_splitter_test_on_multiple_combos() {
    let InterchainTestEnv {
        juno,
        osmosis,
        andromeda,
        interchain,
        ..
    } = InterchainTestEnv::new();

    let chain_combos = ChainMap {
        chains: vec![
            (&osmosis, &juno),
            (&juno, &osmosis),
            (&andromeda, &juno),
        ],
    };

    for (chain1, chain2) in &chain_combos.chains {
        println!("Running test for chain1: {} and chain2: {} combo", 
                chain1.chain_name, chain2.chain_name);

        let contract = SplitterContract::new(chain1.chain.clone());
        
        let deployed_contract = deploy_splitter!(
            contract,
            chain1,
            &InstantiateMsg {
                recipients: vec![
                    AddressPercent {
                        recipient: Recipient {
                            address: AndrAddr::from_string(chain1.addresses[0].clone()),
                            msg: None,
                            ibc_recovery_address: None,
                        },
                        percent: Decimal::percent(60),
                    },
                    AddressPercent {
                        recipient: Recipient {
                            address: AndrAddr::from_string(chain1.addresses[1].clone()),
                            msg: None,
                            ibc_recovery_address: None,
                        },
                        percent: Decimal::percent(40),
                    },
                ],
                kernel_address: chain1.aos.kernel.address().unwrap().into_string(),
                owner: None,
                lock_time: None,
                default_recipient: None,
            },
            "splitter"
        );

        // Now use deployed_contract for the address
        let chain1_recipient = AndrAddr::from_string(format!(
            "ibc://{}/{}",
            chain1.chain_name,
            deployed_contract.address().unwrap()
        ));
        
        let message = AMPMsg::new(
            chain1_recipient,
            to_json_binary(&andromeda_finance::splitter::ExecuteMsg::Send { config: None }).unwrap(),
            Some(vec![Coin {
                denom: chain2.denom.clone(),
                amount: Uint128::new(100),
            }]),
        );


        // Send funds from chain2
        let chain2_send_request = chain2
            .aos
            .kernel
            .execute(
                &os::kernel::ExecuteMsg::Send { message },
                Some(&[Coin {
                    denom: chain2.denom.clone(),
                    amount: Uint128::new(100),
                }]),
            )
            .unwrap();

        let packet_lifetime = interchain
            .await_packets(&chain2.chain_name, chain2_send_request)
            .unwrap();
        ensure_packet_success(packet_lifetime);


        let ibc_denom = format!(
            "ibc/{}/{}",
            chain1.aos.get_aos_channel(&chain2.chain_name).unwrap().direct.unwrap(),
            chain2.chain_name.clone()
        );

        // Setup trigger 
        chain2.aos
            .kernel
            .execute(
                &os::kernel::ExecuteMsg::UpsertKeyAddress {
                    key: "trigger_key".to_string(),
                    value: DEFAULT_SENDER.to_string(),
                },
                None,
            )
            .unwrap();

        let packet_ack = make_ack_success();
        let channel_id = chain2.aos.get_aos_channel(chain1.chain_name.clone()).unwrap().ics20.unwrap();
        
        // Trigger split execution
        let kernel_chain2_splitter = chain2
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
            .await_packets(&chain2.chain_name, kernel_chain2_splitter)
            .unwrap();
        ensure_packet_success(packet_lifetime);

        // Verify split amounts
        let balance1 = chain1.chain.query_all_balances(chain1.addresses[0].clone()).unwrap();
        let balance2 = chain1.chain.query_all_balances(chain1.addresses[1].clone()).unwrap();
        
        assert_eq!(balance1[0].denom, ibc_denom);
        assert_eq!(balance2[0].denom, ibc_denom);
        assert_eq!(balance1[0].amount, Uint128::new(60)); // 60%
        assert_eq!(balance2[0].amount, Uint128::new(40)); // 40%
    }
}



#[test]
fn test_splitter_ibc_update_recipients() {
    let InterchainTestEnv {
        juno,
        osmosis,
        interchain,
        ..
    } = InterchainTestEnv::new();

    let recipient1 = "osmo1qzskhrca90qy2yjjxqzq4yajy842x7c50xq33d";
    let recipient2 = "osmo1v9jxgu33ta047h6lxa803d0j3qqwq2p4k0ahvu";

    let splitter_osmosis = SplitterContract::new(osmosis.chain.clone());
    splitter_osmosis.upload().unwrap();

    splitter_osmosis
        .instantiate(
            &InstantiateMsg {
                recipients: vec![
                    AddressPercent {
                        recipient: Recipient{
                            address: AndrAddr::from_string(recipient1),
                            msg: None,
                            ibc_recovery_address: None,
                        },
                        percent: Decimal::percent(60),
                    },
                    AddressPercent {
                        recipient: Recipient{
                            address: AndrAddr::from_string(recipient2),
                            msg: None,
                            ibc_recovery_address: None,
                        },
                        percent: Decimal::percent(40),
                    },
                ],
                kernel_address: osmosis.aos.kernel.address().unwrap().into_string(),
                owner: None,
                lock_time: None,
                default_recipient: None,
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
                version: "1.0.0".to_string(),
                publisher: None,
                action_fees: None,
            },
            None,
        )
        .unwrap();


 
    let updated_recipients = andromeda_finance::splitter::ExecuteMsg::UpdateRecipients{
        recipients: vec![
            AddressPercent {
                recipient: Recipient{
                    address: AndrAddr::from_string(recipient1),
                    msg: None,
                    ibc_recovery_address: None,
                },
                percent: Decimal::percent(50),
            },
            AddressPercent {
                recipient: Recipient{
                    address: AndrAddr::from_string(recipient2),
                    msg: None,
                    ibc_recovery_address: None,
                },
                percent: Decimal::percent(50),
            },
        ],
    };

    let splitter_addr = splitter_osmosis.address().unwrap();
    let osmosis_recipient = AndrAddr::from_string(format!(
        "ibc://{}/{}",
        osmosis.chain_name,
        splitter_addr
    ));

    let ibc_update_msg = AMPMsg::new(
        osmosis_recipient,
        to_json_binary(&updated_recipients).unwrap(),
        Some(vec![]),
    );

    // 5) Send the IBC message from Juno.
    let kernel_tx = juno.aos.kernel.execute(
        &os::kernel::ExecuteMsg::Send {
            message: ibc_update_msg,
        },
        None,
    ).unwrap();
    

    let packets = interchain
        .await_packets("juno", kernel_tx)
        .unwrap_err();

    assert_eq!(format!("{:?}", packets).contains("error"), true);
}

