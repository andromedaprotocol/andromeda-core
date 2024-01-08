#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query, reply};
use andromeda_finance::splitter::{AddressPercent, ExecuteMsg, InstantiateMsg};
use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_splitter() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query).with_reply(reply);
    Box::new(contract)
}

pub fn mock_splitter_instantiate_msg(
    recipients: Vec<AddressPercent>,
    kernel_address: impl Into<String>,
    lock_time: Option<u64>,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        recipients,
        lock_time,
        kernel_address: kernel_address.into(),
        owner,
    }
}

pub fn mock_splitter_send_msg() -> ExecuteMsg {
    ExecuteMsg::Send {}
}
