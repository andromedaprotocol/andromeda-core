#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use amp::messages::{AMPMsg, AMPPkt, ReplyGas};
use andromeda_finance::splitter::{AddressPercent, ExecuteMsg, InstantiateMsg};
use cosmwasm_std::{coin, to_binary, Empty, ReplyOn};
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_splitter() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_splitter_instantiate_msg(
    recipients: Vec<AddressPercent>,
    kernel_address: impl Into<String>,
    lock_time: Option<u64>,
) -> InstantiateMsg {
    InstantiateMsg {
        recipients,
        lock_time,
        modules: None,
        kernel_address: Some(kernel_address.into()),
    }
}

pub fn mock_splitter_send_msg(packet: Option<AMPPkt>) -> ExecuteMsg {
    ExecuteMsg::Send {
        reply_gas: ReplyGas {
            reply_on: None,
            gas_limit: None,
        },
        packet,
        // packet: Some(AMPPkt::new(
        //     "owner".to_string(),
        //     "previous_sender".to_string(),
        //     vec![AMPMsg {
        //         recipient: "contract10".to_string(),
        //         message: to_binary(&"eyJzZW5kIjp7InJlcGx5X2dhcyI6eyJyZXBseV9vbiI6bnVsbCwiZ2FzX2xpbWl0IjpudWxsfSwicGFja2V0IjpudWxsfX0=").unwrap(),
        //         funds: vec![coin(300, "uandr")],
        //         reply_on: ReplyOn::Never,
        //         gas_limit: None,
        //     }],
        // )),
    }
}
