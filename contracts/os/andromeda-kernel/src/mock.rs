#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query, reply};
use andromeda_std::os::kernel::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_kernel() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query).with_reply(reply);
    Box::new(contract)
}

pub fn mock_kernel_instantiate_message(owner: Option<String>) -> InstantiateMsg {
    InstantiateMsg {
        owner,
        chain_name: "test".to_string(),
    }
}

pub fn mock_upsert_key_address(key: impl Into<String>, value: impl Into<String>) -> ExecuteMsg {
    ExecuteMsg::UpsertKeyAddress {
        key: key.into(),
        value: value.into(),
    }
}

pub fn mock_get_key_address(key: impl Into<String>) -> QueryMsg {
    QueryMsg::KeyAddress { key: key.into() }
}
