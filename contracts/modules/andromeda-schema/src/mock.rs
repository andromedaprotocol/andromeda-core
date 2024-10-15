#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query};
use andromeda_modules::schema::{ExecuteMsg, InstantiateMsg, QueryMsg, ValidateDataResponse};
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Coin, Empty};
use cw_multi_test::{Contract, ContractWrapper};

pub struct MockSchema(Addr);
mock_ado!(MockSchema, ExecuteMsg, QueryMsg);

impl MockSchema {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        kernel_address: String,
        owner: Option<String>,
        schema_json_string: String,
    ) -> MockSchema {
        let msg = mock_schema_instantiate_msg(kernel_address, owner, schema_json_string);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Schema Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockSchema(Addr::unchecked(addr))
    }

    pub fn execute_update_schema(
        &self,
        app: &mut MockApp,
        sender: Addr,
        funds: Option<Coin>,
        new_schema_json_string: String,
    ) -> ExecuteResult {
        let msg = ExecuteMsg::UpdateSchema {
            new_schema_json_string,
        };
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn query_validate_data(&self, app: &mut MockApp, data: String) -> ValidateDataResponse {
        let msg = QueryMsg::ValidateData { data };
        let res: ValidateDataResponse = self.query(app, msg);
        res
    }
}

pub fn mock_andromeda_schema() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_schema_instantiate_msg(
    kernel_address: impl Into<String>,
    owner: Option<String>,
    schema_json_string: String,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address: kernel_address.into(),
        owner,
        schema_json_string,
    }
}
