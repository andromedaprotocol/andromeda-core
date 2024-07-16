#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_data_storage::primitive::{
    ExecuteMsg, GetTypeResponse, GetValueResponse, InstantiateMsg, Primitive, PrimitiveRestriction,
    QueryMsg,
};
use andromeda_std::ado_base::rates::{Rate, RatesMessage};
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Coin, Empty};
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
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = mock_store_value_msg(key, value);
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn execute_add_rate(
        &self,
        app: &mut MockApp,
        sender: Addr,
        action: String,
        rates: Vec<Rate>,
    ) -> ExecuteResult {
        self.execute(app, &mock_set_rate_msg(action, rates), sender, &[])
    }

    pub fn query_value(&self, app: &mut MockApp, key: Option<String>) -> GetValueResponse {
        let msg = mock_primitive_get_value(key);
        let res: GetValueResponse = self.query(app, msg);
        res
    }

    pub fn query_type(&self, app: &mut MockApp, key: Option<String>) -> GetTypeResponse {
        let msg = mock_primitive_get_type(key);
        let res: GetTypeResponse = self.query(app, msg);
        res
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

pub fn mock_set_rate_msg(action: String, rates: Vec<Rate>) -> ExecuteMsg {
    ExecuteMsg::Rates(RatesMessage::SetRate { action, rates })
}

pub fn mock_primitive_get_value(key: Option<String>) -> QueryMsg {
    QueryMsg::GetValue { key }
}

pub fn mock_primitive_get_type(key: Option<String>) -> QueryMsg {
    QueryMsg::GetType { key }
}
