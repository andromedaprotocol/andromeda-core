#![cfg(not(target_arch = "wasm32"))]

use andromeda_fixed_amount_splitter::FixedAmountSplitterContract;
use andromeda_proxy::ProxyContract;
use andromeda_socket::proxy::InitParams;
use andromeda_std::{
    amp::{messages::AMPMsg, AndrAddr, Recipient},
    os,
};
use andromeda_testing::{interchain::ensure_packet_success, InterchainTestEnv};
use cosmwasm_std::{to_json_binary, Coin, Uint128};
use cw_orch::prelude::*;
use cw_orch_interchain::prelude::*;

#[test]
fn test_proxy_ibc() {
    let InterchainTestEnv {
        mut juno,
        osmosis,
        interchain,
        ..
    } = InterchainTestEnv::new();

    let owner_on_osmosis = osmosis.chain.addr_make("ownerosmo");
    let owner_on_juno = juno.chain.addr_make("ownerjuno");

    // Deploy on Osmosis
    let proxy_osmosis = ProxyContract::new(osmosis.chain.clone());
    proxy_osmosis.upload().unwrap();

    // This contract will eventually be instantiated by the proxy contract
    let splitter_osmosis = FixedAmountSplitterContract::new(osmosis.chain.clone());
    splitter_osmosis.upload().unwrap();

    let admins = vec![owner_on_juno.to_string(), owner_on_osmosis.to_string()];
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
    let proxy_osmosis_recipient = AndrAddr::from_string(format!(
        "ibc://{}/{}",
        osmosis.chain_name,
        proxy_osmosis.address().unwrap()
    ));

    let splitter_init_msg = andromeda_finance::fixed_amount_splitter::InstantiateMsg {
        recipients: vec![andromeda_finance::fixed_amount_splitter::AddressAmount {
            recipient: Recipient {
                address: AndrAddr::from_string(owner_on_osmosis.to_string()),
                msg: None,
                ibc_recovery_address: None,
            },
            coins: vec![Coin {
                denom: "osmo".to_string(),
                amount: Uint128::new(100),
            }],
        }],
        default_recipient: None,
        lock_time: None,
        kernel_address: osmosis.aos.kernel.address().unwrap().into_string(),
        owner: None,
    };

    let message = AMPMsg::new(
        proxy_osmosis_recipient,
        to_json_binary(&andromeda_socket::proxy::ExecuteMsg::Instantiate {
            init_params: InitParams::CodeId(splitter_osmosis.code_id().unwrap()),
            message: to_json_binary(&splitter_init_msg).unwrap(),
            admin: None,
            label: None,
        })
        .unwrap(),
        None,
    );

    // Execute IBC msg from Juno
    juno.aos.kernel.set_sender(&owner_on_juno);
    let kernel_juno_send_request = juno
        .aos
        .kernel
        .execute(&os::kernel::ExecuteMsg::Send { message }, &[])
        .unwrap();

    splitter_osmosis.addr_str().unwrap_err();

    // Wait for packet processing
    let packet_lifetime = interchain
        .await_packets(&juno.chain_id, kernel_juno_send_request)
        .unwrap();
    ensure_packet_success(packet_lifetime);
}
