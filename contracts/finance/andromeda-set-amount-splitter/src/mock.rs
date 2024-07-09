#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query, reply};
use andromeda_finance::set_amount_splitter::{AddressAmount, ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::common::expiration::Expiry;
use andromeda_testing::{
    mock::MockApp, mock_ado, mock_contract::ExecuteResult, MockADO, MockContract,
};
use cosmwasm_std::{Addr, Coin, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockSetAmountSplitter(Addr);
mock_ado!(MockSetAmountSplitter, ExecuteMsg, QueryMsg);

impl MockSetAmountSplitter {
    pub fn instantiate(
        app: &mut MockApp,
        code_id: u64,
        sender: Addr,
        recipients: Vec<AddressAmount>,
        kernel_address: impl Into<String>,
        lock_time: Option<Expiry>,
        owner: Option<String>,
    ) -> Self {
        let msg =
            mock_set_amount_splitter_instantiate_msg(recipients, kernel_address, lock_time, owner);
        let res = app.instantiate_contract(code_id, sender, &msg, &[], "Andromeda Splitter", None);

        Self(res.unwrap())
    }

    pub fn execute_send(&self, app: &mut MockApp, sender: Addr, funds: &[Coin]) -> ExecuteResult {
        let msg = mock_set_amount_splitter_send_msg();

        self.execute(app, &msg, sender, funds)
    }
}

pub fn mock_andromeda_set_amount_splitter() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query).with_reply(reply);
    Box::new(contract)
}

pub fn mock_set_amount_splitter_instantiate_msg(
    recipients: Vec<AddressAmount>,
    kernel_address: impl Into<String>,
    lock_time: Option<Expiry>,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        recipients,
        lock_time,
        kernel_address: kernel_address.into(),
        owner,
    }
}

pub fn mock_set_amount_splitter_send_msg() -> ExecuteMsg {
    ExecuteMsg::Send {}
}
