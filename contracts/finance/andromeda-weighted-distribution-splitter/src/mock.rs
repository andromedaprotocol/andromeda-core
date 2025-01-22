#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query, reply};
use andromeda_finance::weighted_splitter::{AddressWeight, ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{amp::Recipient, common::expiration::Expiry};
use andromeda_testing::{
    mock::MockApp, mock_ado, mock_contract::ExecuteResult, MockADO, MockContract,
};
use cosmwasm_std::{Addr, Coin, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockWeightedDistributionSplitter(Addr);
mock_ado!(MockWeightedDistributionSplitter, ExecuteMsg, QueryMsg);

impl MockWeightedDistributionSplitter {
    #[allow(clippy::too_many_arguments)]
    pub fn instantiate(
        app: &mut MockApp,
        code_id: u64,
        sender: Addr,
        recipients: Vec<AddressWeight>,
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
        config: Option<Vec<AddressWeight>>,
    ) -> ExecuteResult {
        let msg = mock_splitter_send_msg(config);

        self.execute(app, &msg, sender, funds)
    }

    pub fn execute_update_recipients(
        &self,
        app: &mut MockApp,
        sender: Addr,
        funds: &[Coin],
        recipients: Vec<AddressWeight>,
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
    recipients: Vec<AddressWeight>,
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

pub fn mock_splitter_send_msg(config: Option<Vec<AddressWeight>>) -> ExecuteMsg {
    ExecuteMsg::Send { config }
}

pub fn mock_splitter_update_recipients_msg(recipients: Vec<AddressWeight>) -> ExecuteMsg {
    ExecuteMsg::UpdateRecipients { recipients }
}
