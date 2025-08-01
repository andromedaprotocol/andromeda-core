#![cfg(not(target_arch = "wasm32"))]
use andromeda_finance::splitter::{AddressPercent, InstantiateMsg};
use andromeda_kernel::ack::make_ack_success;
use andromeda_splitter::SplitterContract;
use andromeda_std::amp::messages::AMPMsgConfig;
use andromeda_std::{
    amp::{messages::AMPMsg, recipient::Recipient, AndrAddr},
    os::{self},
};
use andromeda_testing::{
    ado_deployer, interchain::ensure_packet_success, register_ado, InterchainTestEnv,
};
use cosmwasm_std::{to_json_binary, Coin, Decimal, Uint128};
use cw_orch::mock::cw_multi_test::MockApiBech32;
use cw_orch::mock::MockBase;
use cw_orch::prelude::*;
use cw_orch_interchain::prelude::*;
use rstest::*;
use std::vec;

ado_deployer!(
    deploy_splitter,
    SplitterContract<MockBase<MockApiBech32>>,
    &InstantiateMsg
);

#[rstest]
#[case::osmosis_to_juno("osmosis", "juno")]
#[case::juno_to_osmosis("juno", "osmosis")]
#[case::andromeda_to_juno("andromeda", "juno")]
fn run_splitter_test_on_multiple_combos(#[case] chain1_name: &str, #[case] chain2_name: &str) {
    use cw_orch::mock::cw_multi_test::ibc::types::keccak256;

    let InterchainTestEnv {
        juno,
        osmosis,
        andromeda,
        interchain,
        ..
    } = InterchainTestEnv::new();

    let chains = [
        ("juno", &juno),
        ("osmosis", &osmosis),
        ("andromeda", &andromeda),
    ]
    .into_iter()
    .collect::<std::collections::HashMap<_, _>>();

    let chain1 = chains.get(chain1_name).unwrap();
    let chain2 = chains.get(chain2_name).unwrap();

    let contract = SplitterContract::new(chain1.chain.clone());
    contract.upload().unwrap();
    register_ado!(chain1, contract, "splitter");

    let recipient1 = chain1.chain.addr_make("recipient1");
    let recipient2 = chain1.chain.addr_make("recipient2");

    let deployed_contract = deploy_splitter!(
        contract,
        &InstantiateMsg {
            recipients: Some(vec![
                AddressPercent {
                    recipient: Recipient {
                        address: AndrAddr::from_string(recipient1.clone()),
                        msg: None,
                        ibc_recovery_address: None,
                    },
                    percent: Decimal::percent(60),
                },
                AddressPercent {
                    recipient: Recipient {
                        address: AndrAddr::from_string(recipient2.clone()),
                        msg: None,
                        ibc_recovery_address: None,
                    },
                    percent: Decimal::percent(40),
                },
            ]),
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
            &[Coin {
                denom: chain2.denom.clone(),
                amount: Uint128::new(100),
            }],
        )
        .unwrap();
    let packet_lifetime = interchain
        .await_packets(&chain2.chain_id, chain2_send_request)
        .unwrap();
    ensure_packet_success(packet_lifetime);

    let denom_path = format!(
        "{}/{}",
        chain1
            .aos
            .get_aos_channel(&chain2.chain_name)
            .unwrap()
            .ics20
            .unwrap(),
        chain2.denom.clone()
    );
    let expected_denom = format!("ibc/{}", hex::encode(keccak256(denom_path.as_bytes())));

    // Setup trigger
    chain2
        .aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::UpsertKeyAddress {
                key: "trigger_key".to_string(),
                value: chain2.chain.sender.to_string(),
            },
            &[],
        )
        .unwrap();

    let packet_ack = make_ack_success();
    let channel_id = chain2
        .aos
        .get_aos_channel(chain1.chain_name.clone())
        .unwrap()
        .ics20
        .unwrap();

    // Trigger split execution
    let kernel_chain2_splitter = chain2
        .aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::TriggerRelay {
                packet_sequence: 1,
                packet_ack: packet_ack.clone(),
                channel_id: channel_id.clone(),
            },
            &[],
        )
        .unwrap();

    let packet_lifetime = interchain
        .await_packets(&chain2.chain_id, kernel_chain2_splitter)
        .unwrap();
    ensure_packet_success(packet_lifetime);

    // Verify split amounts
    let balance1 = chain1.chain.query_all_balances(&recipient1).unwrap();

    let balance2 = chain1.chain.query_all_balances(&recipient2).unwrap();

    assert_eq!(balance1[0].denom, expected_denom);
    assert_eq!(balance2[0].denom, expected_denom);
    assert_eq!(balance1[0].amount, Uint128::new(60)); // 60%
    assert_eq!(balance2[0].amount, Uint128::new(40)); // 40%
}

#[test]
fn test_splitter_ibc_update_recipients() {
    let InterchainTestEnv {
        juno,
        osmosis,
        interchain,
        ..
    } = InterchainTestEnv::new();

    let recipient1 = osmosis.chain.addr_make("recipient_1").to_string();
    let recipient2 = osmosis.chain.addr_make("recipient_2").to_string();

    let splitter_osmosis = SplitterContract::new(osmosis.chain.clone());
    splitter_osmosis.upload().unwrap();

    splitter_osmosis
        .instantiate(
            &InstantiateMsg {
                recipients: Some(vec![
                    AddressPercent {
                        recipient: Recipient {
                            address: AndrAddr::from_string(&recipient1),
                            msg: None,
                            ibc_recovery_address: None,
                        },
                        percent: Decimal::percent(60),
                    },
                    AddressPercent {
                        recipient: Recipient {
                            address: AndrAddr::from_string(&recipient2),
                            msg: None,
                            ibc_recovery_address: None,
                        },
                        percent: Decimal::percent(40),
                    },
                ]),
                kernel_address: osmosis.aos.kernel.address().unwrap().into_string(),
                owner: None,
                lock_time: None,
                default_recipient: None,
            },
            None,
            &[],
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
            &[],
        )
        .unwrap();

    let recipients = vec![
        AddressPercent {
            recipient: Recipient {
                address: recipient1.into(),
                msg: None,
                ibc_recovery_address: None,
            },
            percent: Decimal::percent(50),
        },
        AddressPercent {
            recipient: Recipient {
                address: recipient2.into(),
                msg: None,
                ibc_recovery_address: None,
            },
            percent: Decimal::percent(50),
        },
    ];

    let kernel_tx = juno
        .aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::Send {
                message: AMPMsg {
                    recipient: AndrAddr::from_string(format!(
                        "ibc://osmosis/{}",
                        splitter_osmosis.address().unwrap()
                    )),
                    message: to_json_binary(
                        &andromeda_splitter::mock::mock_splitter_update_recipients_msg(Some(
                            recipients,
                        )),
                    )
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
            &[],
        )
        .unwrap();

    let packets = interchain.await_packets("juno-1", kernel_tx).unwrap_err();
    assert!(packets.to_string().contains("error"));
}
