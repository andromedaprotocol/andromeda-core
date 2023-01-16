#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_finance::splitter::{ExecuteMsg, InstantiateMsg, UpdatedAddressPercent};
use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_splitter() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_splitter_instantiate_msg(
    recipients: Vec<UpdatedAddressPercent>,
    _kernel_address: String,
    lock_time: Option<u64>,
) -> InstantiateMsg {
    InstantiateMsg {
        recipients,
        lock_time,
        modules: None,
        kernel_address: None,
    }
}

/// Used to generate a message to store a Code ID
pub fn mock_splitter_send_msg() -> ExecuteMsg {
    ExecuteMsg::Send {}
}
