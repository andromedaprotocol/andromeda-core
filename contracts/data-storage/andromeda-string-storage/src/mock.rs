#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query};
use andromeda_data_storage::string_storage::{
    ExecuteMsg, GetDataOwnerResponse, GetValueResponse, InstantiateMsg, QueryMsg, StringStorage,
    StringStorageRestriction,
};
use andromeda_std::ado_base::rates::{Rate, RatesMessage};
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Coin, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockStringStorage(Addr);
mock_ado!(MockStringStorage, ExecuteMsg, QueryMsg);

impl MockStringStorage {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        kernel_address: String,
        owner: Option<String>,
        restriction: StringStorageRestriction,
    ) -> MockStringStorage {
        let msg = mock_string_storage_instantiate_msg(kernel_address, owner, restriction);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "String Storage Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockStringStorage(Addr::unchecked(addr))
    }

    pub fn execute_set_value(
        &self,
        app: &mut MockApp,
        sender: Addr,
        value: StringStorage,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = mock_store_value_msg(value);
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
        rate: Rate,
    ) -> ExecuteResult {
        self.execute(app, &mock_set_rate_msg(action, rate), sender, &[])
    }

    pub fn query_value(&self, app: &mut MockApp) -> GetValueResponse {
        let msg = mock_string_storage_get_value();
        let res: GetValueResponse = self.query(app, msg);
        res
    }

    pub fn query_data_owner(&self, app: &mut MockApp) -> GetDataOwnerResponse {
        let msg = mock_string_storage_get_data_owner();
        let res: GetDataOwnerResponse = self.query(app, msg);
        res
    }
}

pub fn mock_andromeda_string_storage() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_string_storage_instantiate_msg(
    kernel_address: String,
    owner: Option<String>,
    restriction: StringStorageRestriction,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address,
        owner,
        restriction,
    }
}

/// Used to generate a message to store a string storage value
pub fn mock_store_value_msg(value: StringStorage) -> ExecuteMsg {
    ExecuteMsg::SetValue { value }
}

pub fn mock_set_rate_msg(action: String, rate: Rate) -> ExecuteMsg {
    ExecuteMsg::Rates(RatesMessage::SetRate { action, rate })
}

pub fn mock_string_storage_get_value() -> QueryMsg {
    QueryMsg::GetValue {}
}

pub fn mock_string_storage_get_data_owner() -> QueryMsg {
    QueryMsg::GetDataOwner {}
}
