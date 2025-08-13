#![cfg(not(target_arch = "wasm32"))]

use andromeda_kernel::ack::make_ack_success;
use andromeda_proxy::ProxyContract;
use andromeda_std::{
    amp::{messages::AMPMsg, AndrAddr, Recipient},
    os,
};
use andromeda_testing::{interchain::ensure_packet_success, InterchainTestEnv};
use cosmwasm_std::{to_json_binary, Coin, Uint128};
use cw_orch::{mock::cw_multi_test::ibc::types::keccak256, prelude::*};
use cw_orch_interchain::prelude::*;

#[test]
fn test_fixed_amount_splitter_ibc() {
    let InterchainTestEnv {
        juno,
        osmosis,
        interchain,
        ..
    } = InterchainTestEnv::new();

    let owner_on_osmosis = osmosis.chain.addr_make("ownerosmo");
    let owner_on_juno = juno.chain.addr_make("ownerjuno");

    // Deploy on Osmosis
    let proxy_osmosis = ProxyContract::new(osmosis.chain.clone());
    proxy_osmosis.upload().unwrap();

    let admins = vec![owner_on_juno.to_string()];
    // Owner on osmosis will init the proxy on osmo, and set his juno address as admin
    proxy_osmosis
        .instantiate(
            &andromeda_socket::proxy::InstantiateMsg {
                admins: admins,
                kernel_address: osmosis.aos.kernel.address().unwrap().into_string(),
                owner: None,
            },
            None,
            &[],
        )
        .unwrap();

    // Register contract
    osmosis
        .aos
        .adodb
        .execute(
            &os::adodb::ExecuteMsg::Publish {
                code_id: proxy_osmosis.code_id().unwrap(),
                ado_type: "proxy".to_string(),
                action_fees: None,
                version: "0.1.0".to_string(),
                publisher: None,
            },
            &[],
        )
        .unwrap();

    // Create IBC message
    let osmosis_recipient = AndrAddr::from_string(format!(
        "ibc://{}/{}",
        osmosis.chain_name,
        proxy_osmosis.address().unwrap()
    ));

    let message = AMPMsg::new(
        osmosis_recipient,
        to_json_binary(
            &andromeda_finance::fixed_amount_splitter::ExecuteMsg::Send { config: None },
        )
        .unwrap(),
        None,
    );

    // Execute IBC transfer from Juno
    let kernel_juno_send_request = juno
        .aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::Send { message },
            &[Coin {
                amount: Uint128::new(100000000),
                denom: juno.denom.clone(),
            }],
        )
        .unwrap();

    // Wait for packet processing
    let packet_lifetime = interchain
        .await_packets(&juno.chain_id, kernel_juno_send_request)
        .unwrap();
    ensure_packet_success(packet_lifetime);

    // Check balances
    let balances = osmosis
        .chain
        .query_all_balances(&osmosis.aos.kernel.address().unwrap())
        .unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].denom, expected_denom.clone());
    assert_eq!(balances[0].amount.u128(), 100000000);

    // Setup trigger
    juno.aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::UpsertKeyAddress {
                key: "trigger_key".to_string(),
                value: juno.chain.sender.to_string(),
            },
            &[],
        )
        .unwrap();

    let packet_ack = make_ack_success();
    let channel_id = juno
        .aos
        .get_aos_channel(&osmosis.chain_name)
        .unwrap()
        .ics20
        .unwrap();

    // Execute trigger relay
    let kernel_juno_splitter_request = juno
        .aos
        .kernel
        .execute(
            &os::kernel::ExecuteMsg::TriggerRelay {
                packet_sequence: 1,
                packet_ack,
                channel_id,
            },
            &[],
        )
        .unwrap();

    let packet_lifetime = interchain
        .await_packets(&juno.chain_id, kernel_juno_splitter_request)
        .unwrap();
    ensure_packet_success(packet_lifetime);

    // Verify final recipient balance
    let balances = osmosis.chain.query_all_balances(&recipient).unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].denom, expected_denom);
    assert_eq!(balances[0].amount.u128(), 100);
}
