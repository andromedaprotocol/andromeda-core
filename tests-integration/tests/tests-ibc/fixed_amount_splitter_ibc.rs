#![cfg(not(target_arch = "wasm32"))]

use andromeda_fixed_amount_splitter::FixedAmountSplitterContract;
use andromeda_kernel::ack::make_ack_success;
use andromeda_std::{
    amp::{messages::AMPMsg, AndrAddr, Recipient},
    os,
};
use andromeda_testing::{
    interchain::{ensure_packet_success, DEFAULT_SENDER},
    InterchainTestEnv,
};
use cosmwasm_std::{to_json_binary, Coin, Uint128};
use cw_orch::prelude::*;
use cw_orch_interchain::prelude::*;

#[test]
fn test_fixed_amount_splitter_ibc() {
    let InterchainTestEnv {
        juno,
        osmosis,
        interchain,
        ..
    } = InterchainTestEnv::new();

    let recipient = osmosis.chain.addr_make("recipient");

    let splitter_osmosis = FixedAmountSplitterContract::new(osmosis.chain.clone());
    splitter_osmosis.upload().unwrap();

    splitter_osmosis
        .instantiate(
            &andromeda_finance::fixed_amount_splitter::InstantiateMsg {
                recipients: vec![andromeda_finance::fixed_amount_splitter::AddressAmount {
                    recipient: Recipient {
                        address: AndrAddr::from_string(recipient.clone()),
                        msg: None,
                        ibc_recovery_address: None,
                    },
                    coins: vec![Coin {
                        denom: "ibc/channel-0/juno".to_string(),
                        amount: Uint128::new(100),
                    }],
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
                ado_type: "fixed-amount-splitter".to_string(),
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
        to_json_binary(
            &andromeda_finance::fixed_amount_splitter::ExecuteMsg::Send { config: None },
        )
        .unwrap(),
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
                value: DEFAULT_SENDER.to_string(),
            },
            None,
        )
        .unwrap();

    let packet_ack = make_ack_success();

    let channel_id = juno.aos.get_aos_channel("osmosis").unwrap().ics20.unwrap();
    // Construct an Execute msg from the kernel on juno intended for the splitter on osmosis
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
        .query_all_balances(&osmosis.chain.addr_make(recipient))
        .unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].denom, ibc_denom);
    assert_eq!(balances[0].amount.u128(), 100);
}
