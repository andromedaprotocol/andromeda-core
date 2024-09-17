#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_data_storage::matrix::{
    ExecuteMsg, GetMatrixResponse, InstantiateMsg, Matrix, MatrixRestriction, QueryMsg,
};
use andromeda_std::{
    ado_base::rates::{Rate, RatesMessage},
    amp::AndrAddr,
};
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Coin, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockMatrix(Addr);
mock_ado!(MockMatrix, ExecuteMsg, QueryMsg);

impl MockMatrix {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        kernel_address: String,
        owner: Option<String>,
        restriction: MatrixRestriction,
    ) -> MockMatrix {
        let msg = mock_matrix_instantiate_msg(kernel_address, owner, restriction);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Matrix Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockMatrix(Addr::unchecked(addr))
    }

    pub fn execute_store_matrix(
        &self,
        app: &mut MockApp,
        sender: Addr,
        key: Option<String>,
        data: Matrix,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = mock_store_matrix_msg(key, data);
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

    pub fn execute_update_restriction(
        &self,
        app: &mut MockApp,
        sender: Addr,
        restriction: MatrixRestriction,
    ) -> ExecuteResult {
        self.execute(app, &mock_update_restriction_msg(restriction), sender, &[])
    }

    pub fn execute_delete_matrix(
        &self,
        app: &mut MockApp,
        sender: Addr,
        key: Option<String>,
    ) -> ExecuteResult {
        self.execute(app, &mock_delete_matrix_msg(key), sender, &[])
    }

    pub fn query_matrix(&self, app: &mut MockApp, key: Option<String>) -> GetMatrixResponse {
        let msg = mock_matrix_get_matrix(key);
        let res: GetMatrixResponse = self.query(app, msg);
        res
    }

    pub fn query_all_keys(&self, app: &mut MockApp) -> Vec<String> {
        let msg = QueryMsg::AllKeys {};
        let res: Vec<String> = self.query(app, msg);
        res
    }

    pub fn query_owner_keys(&self, app: &mut MockApp, owner: AndrAddr) -> Vec<String> {
        let msg = QueryMsg::OwnerKeys { owner };
        let res: Vec<String> = self.query(app, msg);
        res
    }
}

pub fn mock_andromeda_matrix() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_matrix_instantiate_msg(
    kernel_address: String,
    owner: Option<String>,
    restriction: MatrixRestriction,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address,
        owner,
        restriction,
    }
}

pub fn mock_store_matrix_msg(key: Option<String>, data: Matrix) -> ExecuteMsg {
    ExecuteMsg::StoreMatrix { key, data }
}

pub fn mock_update_restriction_msg(restriction: MatrixRestriction) -> ExecuteMsg {
    ExecuteMsg::UpdateRestriction { restriction }
}

pub fn mock_delete_matrix_msg(key: Option<String>) -> ExecuteMsg {
    ExecuteMsg::DeleteMatrix { key }
}

pub fn mock_set_rate_msg(action: String, rate: Rate) -> ExecuteMsg {
    ExecuteMsg::Rates(RatesMessage::SetRate { action, rate })
}

pub fn mock_matrix_get_matrix(key: Option<String>) -> QueryMsg {
    QueryMsg::GetMatrix { key }
}
