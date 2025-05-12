#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query, reply};
use andromeda_finance::timelock::{EscrowConditionInput, ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::amp::Recipient;
use andromeda_testing::{
    mock::MockApp, mock_ado, mock_contract::ExecuteResult, MockADO, MockContract,
};
use cosmwasm_std::{Addr, Coin, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockTimelock(Addr);
mock_ado!(MockTimelock, ExecuteMsg, QueryMsg);

impl MockTimelock {
    #[allow(clippy::too_many_arguments)]
    pub fn instantiate(
        app: &mut MockApp,
        code_id: u64,
        sender: Addr,
        kernel_address: impl Into<String>,
        owner: Option<String>,
    ) -> Self {
        let msg = mock_timelock_instantiate_msg(kernel_address, owner);
        let res = app.instantiate_contract(code_id, sender, &msg, &[], "Andromeda timelock", None);

        Self(res.unwrap())
    }

    pub fn execute_hold_funds(
        &self,
        app: &mut MockApp,
        sender: Addr,
        funds: &[Coin],
        condition: Option<EscrowConditionInput>,
        recipient: Option<Recipient>,
    ) -> ExecuteResult {
        let msg = mock_timelock_hold_funds_msg(condition, recipient);

        self.execute(app, &msg, sender, funds)
    }

    pub fn execute_release_funds(
        &self,
        app: &mut MockApp,
        sender: Addr,
        funds: &[Coin],
        recipient_addr: Option<String>,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> ExecuteResult {
        let msg = mock_timelock_release_funds_msg(recipient_addr, start_after, limit);

        self.execute(app, &msg, sender, funds)
    }
}

pub fn mock_andromeda_timelock() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query).with_reply(reply);
    Box::new(contract)
}

pub fn mock_timelock_instantiate_msg(
    kernel_address: impl Into<String>,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address: kernel_address.into(),
        owner,
    }
}

pub fn mock_timelock_hold_funds_msg(
    condition: Option<EscrowConditionInput>,
    recipient: Option<Recipient>,
) -> ExecuteMsg {
    ExecuteMsg::HoldFunds {
        condition,
        recipient,
    }
}

pub fn mock_timelock_release_funds_msg(
    recipient_addr: Option<String>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> ExecuteMsg {
    ExecuteMsg::ReleaseFunds {
        recipient_addr,
        start_after,
        limit,
    }
}
