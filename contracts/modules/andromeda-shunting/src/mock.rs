#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_modules::shunting::{InstantiateMsg, QueryMsg};
use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_shunting() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_shunting_instantiate_msg(
    expressions: Vec<String>,
    kernel_address: impl Into<String>,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        expressions,
        kernel_address: kernel_address.into(),
        owner,
    }
}

pub fn mock_shunting_query_msg(params: Vec<String>) -> QueryMsg {
    QueryMsg::EvalWithParams { params }
}
