#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query};
use andromeda_modules::date_time::{InstantiateMsg, QueryMsg, Timezone};
use andromeda_modules::date_time::GetDateTimeResponse;
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockDateTime(Addr);
mock_ado!(MockDateTime, ExecuteMsg, QueryMsg);

impl MockDateTime {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        kernel_address: String,
        owner: Option<String>,
    ) -> MockDateTime {
        let msg = mock_date_time_instantiate_msg(kernel_address, owner);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Date Time Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockDateTime(Addr::unchecked(addr))
    }

    pub fn query_date_time(&self, app: &mut MockApp, timezone: Timezone) -> GetDateTimeResponse {
        let msg = QueryMsg::GetDateTime { timezone };
        let res: GetDateTimeResponse = self.query(app, msg);
        res
    }
}

pub fn mock_andromeda_date_time() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_date_time_instantiate_msg(
    kernel_address: String,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address,
        owner,
    }
}
