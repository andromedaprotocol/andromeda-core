#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query};
use andromeda_modules::block_info_utils::{InstantiateMsg, QueryMsg};
use andromeda_modules::block_info_utils::GetBlockHeightResponse;
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockBlockInfoUtilsUtils(Addr);
mock_ado!(MockBlockInfoUtilsUtils, ExecuteMsg, QueryMsg);

impl MockBlockInfoUtilsUtils {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        kernel_address: String,
        owner: Option<String>,
    ) -> MockBlockInfoUtilsUtils {
        let msg = mock_block_info_utils_instantiate_msg(kernel_address, owner);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Block Info Utils Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockBlockInfoUtilsUtils(Addr::unchecked(addr))
    }

    pub fn query_block_height(&self, app: &mut MockApp) -> GetBlockHeightResponse {
        let msg = QueryMsg::GetBlockHeight {};
        let res: GetBlockHeightResponse = self.query(app, msg);
        res
    }
}

pub fn mock_andromeda_block_info_utils() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_block_info_utils_instantiate_msg(
    kernel_address: String,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address,
        owner,
    }
}
