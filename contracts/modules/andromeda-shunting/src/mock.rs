#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_modules::shunting::{
    EvaluateParam, ExecuteMsg, InstantiateMsg, QueryMsg, ShuntingResponse,
};
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{Contract, ContractWrapper};

use andromeda_testing::{
    mock::MockApp,
    mock_ado,
    mock_contract::{MockADO, MockContract},
};

pub struct MockShunting(Addr);
mock_ado!(MockShunting, ExecuteMsg, QueryMsg);

impl MockShunting {
    pub fn evaluate(&self, app: &MockApp, params: Vec<EvaluateParam>) -> ShuntingResponse {
        let msg = mock_shunting_evaluate(params);
        let res: ShuntingResponse = self.query(app, msg);
        res
    }
}

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

pub fn mock_shunting_evaluate(params: Vec<EvaluateParam>) -> QueryMsg {
    QueryMsg::Evaluate { params }
}
