#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_data_storage::primitive::{
    ExecuteMsg, InstantiateMsg, Primitive, PrimitiveRestriction, QueryMsg,
};
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockPrimitive(Addr);
mock_ado!(MockPrimitive, ExecuteMsg, QueryMsg);

impl MockPrimitive {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        kernel_address: String,
        owner: Option<String>,
        restriction: PrimitiveRestriction,
    ) -> MockPrimitive {
        let msg = mock_primitive_instantiate_msg(kernel_address, owner, restriction);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Primitive Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockPrimitive(Addr::unchecked(addr))
    }

    pub fn execute_set_value(
        &self,
        app: &mut MockApp,
        sender: Addr,
        key: Option<String>,
        value: Primitive,
    ) -> ExecuteResult {
        let msg = mock_store_value_msg(key, value);
        app.execute_contract(sender, self.addr().clone(), &msg, &[])
    }
}

pub fn mock_andromeda_primitive() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_primitive_instantiate_msg(
    kernel_address: String,
    owner: Option<String>,
    restriction: PrimitiveRestriction,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address,
        owner,
        restriction,
    }
}

/// Used to generate a message to store a primitive value
pub fn mock_store_value_msg(key: Option<String>, value: Primitive) -> ExecuteMsg {
    ExecuteMsg::SetValue { key, value }
}

/// Used to generate a message to store an address, primarily used for the address registry contract
pub fn mock_store_address_msgs(key: String, address: Addr) -> ExecuteMsg {
    ExecuteMsg::SetValue {
        key: Some(key),
        value: Primitive::Addr(address),
    }
}

pub fn mock_primitive_get_value(key: Option<String>) -> QueryMsg {
    QueryMsg::GetValue { key }
}
