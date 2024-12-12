#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query, reply};
use andromeda_finance::splitter::{AddressPercent, ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{amp::Recipient, common::expiration::Expiry};
use andromeda_testing::{
    mock::MockApp, mock_ado, mock_contract::ExecuteResult, MockADO, MockContract,
};
use cosmwasm_std::{Addr, Coin, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockSplitter(Addr);
mock_ado!(MockSplitter, ExecuteMsg, QueryMsg);

impl MockSplitter {
    pub fn instantiate(
        app: &mut MockApp,
        code_id: u64,
        sender: Addr,
        recipients: Vec<AddressPercent>,
        kernel_address: impl Into<String>,
        lock_time: Option<Expiry>,
        owner: Option<String>,
        default_recipient: Option<Recipient>,
    ) -> Self {
        let msg = mock_splitter_instantiate_msg(
            recipients,
            kernel_address,
            lock_time,
            owner,
            default_recipient,
        );
        let res = app.instantiate_contract(code_id, sender, &msg, &[], "Andromeda Splitter", None);

        Self(res.unwrap())
    }

    pub fn execute_send(
        &self,
        app: &mut MockApp,
        sender: Addr,
        funds: &[Coin],
        config: Option<Vec<AddressPercent>>,
    ) -> ExecuteResult {
        let msg = mock_splitter_send_msg(config);

        self.execute(app, &msg, sender, funds)
    }

    pub fn execute_update_recipients(
        &self,
        app: &mut MockApp,
        sender: Addr,
        funds: &[Coin],
        recipients: Vec<AddressPercent>,
    ) -> ExecuteResult {
        let msg = mock_splitter_update_recipients_msg(recipients);

        self.execute(app, &msg, sender, funds)
    }
}

pub fn mock_andromeda_splitter() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query).with_reply(reply);
    Box::new(contract)
}

pub fn mock_splitter_instantiate_msg(
    recipients: Vec<AddressPercent>,
    kernel_address: impl Into<String>,
    lock_time: Option<Expiry>,
    owner: Option<String>,
    default_recipient: Option<Recipient>,
) -> InstantiateMsg {
    InstantiateMsg {
        recipients,
        lock_time,
        kernel_address: kernel_address.into(),
        owner,
        default_recipient,
    }
}

pub fn mock_splitter_send_msg(config: Option<Vec<AddressPercent>>) -> ExecuteMsg {
    ExecuteMsg::Send { config }
}

pub fn mock_splitter_update_recipients_msg(recipients: Vec<AddressPercent>) -> ExecuteMsg {
    ExecuteMsg::UpdateRecipients { recipients }
}
