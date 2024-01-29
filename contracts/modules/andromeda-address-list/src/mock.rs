#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_modules::address_list::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_testing::{mock_ado, mock_contract::ExecuteResult, MockADO, MockContract};
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

pub struct MockAddressList(Addr);
mock_ado!(MockAddressList, ExecuteMsg, QueryMsg);

impl MockAddressList {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut App,
        is_inclusive: bool,
        kernel_address: impl Into<String>,
        owner: Option<String>,
    ) -> MockAddressList {
        let msg = mock_address_list_instantiate_msg(is_inclusive, kernel_address, owner);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Address List Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockAddressList(Addr::unchecked(addr))
    }

    pub fn execute_add_address(
        &self,
        app: &mut App,
        sender: Addr,
        address: impl Into<String>,
    ) -> ExecuteResult {
        self.execute(app, &mock_add_address_msg(address), sender, &[])
    }

    pub fn query_includes_address(&self, app: &App, address: impl Into<String>) -> bool {
        self.query::<bool>(app, mock_includes_address_msg(address))
    }
}

pub fn mock_andromeda_address_list() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_address_list_instantiate_msg(
    is_inclusive: bool,
    kernel_address: impl Into<String>,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        is_inclusive,
        kernel_address: kernel_address.into(),
        owner,
    }
}

pub fn mock_add_address_msg(address: impl Into<String>) -> ExecuteMsg {
    ExecuteMsg::AddAddress {
        address: address.into(),
    }
}

pub fn mock_includes_address_msg(address: impl Into<String>) -> QueryMsg {
    QueryMsg::IncludesAddress {
        address: address.into(),
    }
}
