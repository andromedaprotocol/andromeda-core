#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query};
use andromeda_modules::string_utils::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_modules::string_utils::{
    GetSplitResultResponse, Delimiter,
};
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Coin, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockStringUtils(Addr);
mock_ado!(MockStringUtils, ExecuteMsg, QueryMsg);

impl MockStringUtils {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        kernel_address: String,
        owner: Option<String>,
    ) -> MockStringUtils {
        let msg = mock_string_utils_instantiate_msg(kernel_address, owner);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "String Utils Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockStringUtils(Addr::unchecked(addr))
    }

    pub fn execute_try_split(
        &self,
        app: &mut MockApp,
        sender: Addr,
        input: String,
        delimiter: Delimiter,
        funds:Option<Coin>,
    ) -> ExecuteResult {
        let msg = mock_try_split_msg(input, delimiter);
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn query_split_result(&self, app: &mut MockApp) -> GetSplitResultResponse {
        let msg = QueryMsg::GetSplitResult {};
        let res: GetSplitResultResponse = self.query(app, msg);
        res
    }
}

pub fn mock_andromeda_string_utils() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_string_utils_instantiate_msg(
    kernel_address: String,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address,
        owner,
    }
}

pub fn mock_try_split_msg(input: String, delimiter: Delimiter) -> ExecuteMsg {
    ExecuteMsg::Split { input, delimiter }
}
