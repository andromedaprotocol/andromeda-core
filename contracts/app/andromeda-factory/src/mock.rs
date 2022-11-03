#![cfg(not(target_arch = "wasm32"))]

use crate::contract::{execute, instantiate, query};
use andromeda_app::factory::{InstantiateMsg, ExecuteMsg, QueryMsg};
use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_factory() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_factory_instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {  }
}

/// Used to generate a message to store a Code ID
pub fn mock_store_code_id_msg(code_id_key: String, code_id: u64) -> ExecuteMsg {
    ExecuteMsg::UpdateCodeId { code_id_key, code_id }
}

/// Used to generate a Code ID query message
pub fn mock_get_code_id_msg(code_id_key: String) -> QueryMsg {
    QueryMsg::CodeId { key: code_id_key }
}