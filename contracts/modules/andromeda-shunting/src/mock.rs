#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_modules::address_list::{ExecuteMsg, InstantiateMsg};
use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_address_list() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_address_list_instantiate_msg(
    is_inclusive: bool,
    kernel_address: impl Into<String>,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        is_inclusive,
        kernel_address: kernel_address.into(),
        owner,
    }
}

pub fn mock_add_address_msg(address: impl Into<String>) -> ExecuteMsg {
    ExecuteMsg::AddAddress {
        address: address.into(),
    }
}
