#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query};
use andromeda_data_storage::counter::{
    CounterRestriction, ExecuteMsg, InstantiateMsg, QueryMsg, State,
};
use andromeda_data_storage::counter::{
    GetCurrentAmountResponse, GetDecreaseAmountResponse, GetIncreaseAmountResponse,
    GetInitialAmountResponse, GetRestrictionResponse,
};
use andromeda_std::ado_base::rates::{Rate, RatesMessage};
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Coin, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockCounter(Addr);
mock_ado!(MockCounter, ExecuteMsg, QueryMsg);

impl MockCounter {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        kernel_address: String,
        owner: Option<String>,
        restriction: CounterRestriction,
        initial_amount: Option<u64>,
        increase_amount: Option<u64>,
        decrease_amount: Option<u64>,
    ) -> MockCounter {
        let msg = mock_counter_instantiate_msg(
            kernel_address,
            owner,
            restriction,
            initial_amount,
            increase_amount,
            decrease_amount,
        );
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Counter Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockCounter(Addr::unchecked(addr))
    }

    pub fn execute_increment(
        &self,
        app: &mut MockApp,
        sender: Addr,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = mock_execute_increment_msg();
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn execute_decrement(
        &self,
        app: &mut MockApp,
        sender: Addr,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = mock_execute_increment_msg();
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn execute_reset(
        &self,
        app: &mut MockApp,
        sender: Addr,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = mock_execute_reset_msg();
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn execute_update_restriction(
        &self,
        app: &mut MockApp,
        sender: Addr,
        restriction: CounterRestriction,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = mock_execute_update_restriction_msg(restriction);
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn execute_set_increase_amount(
        &self,
        app: &mut MockApp,
        sender: Addr,
        increase_amount: u64,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = mock_execute_set_increase_amount_msg(increase_amount);
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn execute_set_decrease_amount(
        &self,
        app: &mut MockApp,
        sender: Addr,
        decrease_amount: u64,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = mock_execute_set_decrease_amount_msg(decrease_amount);
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn query_initial_amount(&self, app: &mut MockApp) -> GetInitialAmountResponse {
        let msg = QueryMsg::GetInitialAmount {};
        let res: GetInitialAmountResponse = self.query(app, msg);
        res
    }

    pub fn query_current_amount(&self, app: &mut MockApp) -> GetCurrentAmountResponse {
        let msg = QueryMsg::GetCurrentAmount {};
        let res: GetCurrentAmountResponse = self.query(app, msg);
        res
    }

    pub fn query_increase_amount(&self, app: &mut MockApp) -> GetIncreaseAmountResponse {
        let msg = QueryMsg::GetIncreaseAmount {};
        let res: GetIncreaseAmountResponse = self.query(app, msg);
        res
    }

    pub fn query_decrease_amount(&self, app: &mut MockApp) -> GetDecreaseAmountResponse {
        let msg = QueryMsg::GetDecreaseAmount {};
        let res: GetDecreaseAmountResponse = self.query(app, msg);
        res
    }

    pub fn query_restriction(&self, app: &mut MockApp) -> GetRestrictionResponse {
        let msg = QueryMsg::GetRestriction {};
        let res: GetRestrictionResponse = self.query(app, msg);
        res
    }
}

pub fn mock_andromeda_counter() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_counter_instantiate_msg(
    kernel_address: String,
    owner: Option<String>,
    restriction: CounterRestriction,
    initial_amount: Option<u64>,
    increase_amount: Option<u64>,
    decrease_amount: Option<u64>,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address,
        owner,
        restriction,
        initial_state: State {
            initial_amount,
            increase_amount,
            decrease_amount,
        },
    }
}

pub fn mock_execute_increment_msg() -> ExecuteMsg {
    ExecuteMsg::Increment {}
}

pub fn mock_execute_decrement_msg() -> ExecuteMsg {
    ExecuteMsg::Decrement {}
}

pub fn mock_execute_reset_msg() -> ExecuteMsg {
    ExecuteMsg::Reset {}
}

pub fn mock_execute_update_restriction_msg(restriction: CounterRestriction) -> ExecuteMsg {
    ExecuteMsg::UpdateRestriction { restriction }
}

pub fn mock_execute_set_increase_amount_msg(increase_amount: u64) -> ExecuteMsg {
    ExecuteMsg::SetIncreaseAmount { increase_amount }
}

pub fn mock_execute_set_decrease_amount_msg(decrease_amount: u64) -> ExecuteMsg {
    ExecuteMsg::SetDecreaseAmount { decrease_amount }
}
