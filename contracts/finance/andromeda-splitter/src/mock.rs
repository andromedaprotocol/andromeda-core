#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query, reply};
use andromeda_finance::splitter::{AddressPercent, ExecuteMsg, InstantiateMsg};
use andromeda_testing::{mock_ado, mock_contract::ExecuteResult, MockADO, MockContract};
use cosmwasm_std::{Addr, Coin, Empty};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

pub struct MockSplitter(Addr);
mock_ado!(MockSplitter);

impl MockSplitter {
    pub fn instantiate(
        app: &mut App,
        code_id: u64,
        sender: Addr,
        recipients: Vec<AddressPercent>,
        kernel_address: impl Into<String>,
        lock_time: Option<u64>,
        owner: Option<String>,
    ) -> Self {
        let msg = mock_splitter_instantiate_msg(recipients, kernel_address, lock_time, owner);
        let res = app.instantiate_contract(
            code_id,
            sender.clone(),
            &msg,
            &[],
            "Andromeda Splitter",
            None,
        );

        Self(res.unwrap())
    }

    pub fn execute_send(&self, app: &mut App, sender: Addr, funds: &[Coin]) -> ExecuteResult {
        let msg = mock_splitter_send_msg();
        let res = self.execute(app, &msg, sender, funds);

        res
    }
}

pub fn mock_andromeda_splitter() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query).with_reply(reply);
    Box::new(contract)
}

pub fn mock_splitter_instantiate_msg(
    recipients: Vec<AddressPercent>,
    kernel_address: impl Into<String>,
    lock_time: Option<u64>,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        recipients,
        lock_time,
        kernel_address: kernel_address.into(),
        owner,
    }
}

pub fn mock_splitter_send_msg() -> ExecuteMsg {
    ExecuteMsg::Send {}
}
