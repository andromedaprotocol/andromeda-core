#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_automation::counter::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_counter() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_counter_instantiate_msg(
    kernel_address: impl Into<String>,
    whitelist: Vec<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        whitelist,
        kernel_address: Some(kernel_address.into()),
    }
}

pub fn mock_counter_increment_one_msg() -> ExecuteMsg {
    ExecuteMsg::IncrementOne {}
}

pub fn mock_counter_query_current_count_msg() -> QueryMsg {
    QueryMsg::CurrentCount {}
}
