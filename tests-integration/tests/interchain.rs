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
    os,
};
use andromeda_testing::InterchainTestEnv;
use cosmwasm_std::to_json_binary;
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
