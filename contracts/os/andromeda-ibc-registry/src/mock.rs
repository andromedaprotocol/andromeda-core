#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query};
use andromeda_std::amp::AndrAddr;
use andromeda_std::os::ibc_registry::{
    DenomInfoResponse, ExecuteMsg, IBCDenomInfo, InstantiateMsg, QueryMsg,
};
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Coin, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockIbcRegistry(Addr);
mock_ado!(MockIbcRegistry, ExecuteMsg, QueryMsg);

impl MockIbcRegistry {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        kernel_address: Addr,
        owner: Option<String>,
        service_address: AndrAddr,
    ) -> MockIbcRegistry {
        let msg = mock_ibc_registry_instantiate_msg(kernel_address, owner, service_address);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "IBC Registry Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockIbcRegistry(Addr::unchecked(addr))
    }

    pub fn execute_execute_store_denom_info(
        &self,
        app: &mut MockApp,
        sender: Addr,
        funds: Option<Coin>,
        ibc_denom_info: Vec<IBCDenomInfo>,
    ) -> ExecuteResult {
        let msg = mock_execute_store_denom_info_msg(ibc_denom_info);
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn query_denom_info(&self, app: &mut MockApp, denom: String) -> DenomInfoResponse {
        let msg = QueryMsg::DenomInfo { denom };
        let res: DenomInfoResponse = self.query(app, msg);
        res
    }

    pub fn query_all_denom_info(
        &self,
        app: &mut MockApp,
        limit: Option<u64>,
        start_after: Option<u64>,
    ) -> DenomInfoResponse {
        let msg = QueryMsg::AllDenomInfo { limit, start_after };
        let res: DenomInfoResponse = self.query(app, msg);
        res
    }
}

pub fn mock_andromeda_counter() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_ibc_registry_instantiate_msg(
    kernel_address: Addr,
    owner: Option<String>,
    service_address: AndrAddr,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address,
        owner,
        service_address,
    }
}

pub fn mock_execute_store_denom_info_msg(ibc_denom_info: Vec<IBCDenomInfo>) -> ExecuteMsg {
    ExecuteMsg::StoreDenomInfo { ibc_denom_info }
}
