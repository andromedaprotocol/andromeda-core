#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_finance::splitter::{AddressPercent, ExecuteMsg, InstantiateMsg};
use andromeda_os::messages::{AMPPkt, ReplyGasExit};
use cosmwasm_std::Empty;
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

        kernel_address: Some(kernel_address.into()),
    }
}

pub fn mock_splitter_send_msg(packet: Option<AMPPkt>) -> ExecuteMsg {
    ExecuteMsg::Send {
        reply_gas: ReplyGasExit {
            reply_on: None,
            gas_limit: None,
            exit_at_error: Some(false),
        },
        packet,
    }
}
