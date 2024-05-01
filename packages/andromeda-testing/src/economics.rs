use andromeda_economics::mock::*;
use andromeda_std::{
    amp::AndrAddr,
    os::economics::{ExecuteMsg, QueryMsg},
};
use cosmwasm_std::{Addr, Coin, Uint128};
use cw_multi_test::Executor;

use crate::{mock::MockApp, mock_ado, mock_contract::ExecuteResult, MockADO, MockContract};

pub struct MockEconomics(Addr);
mock_ado!(MockEconomics, ExecuteMsg, QueryMsg);

impl MockEconomics {
    pub fn instantiate(
        app: &mut MockApp,
        code_id: u64,
        sender: Addr,
        owner: Option<String>,
        kernel_address: String,
    ) -> Self {
        let msg = mock_economics_instantiate_msg(kernel_address, owner);
        let res = app.instantiate_contract(
            code_id,
            sender.clone(),
            &msg,
            &[],
            "Economics",
            Some(sender.to_string()),
        );

        Self(res.unwrap())
    }

    pub fn execute_deposit(
        &self,
        app: &mut MockApp,
        sender: Addr,
        address: Option<AndrAddr>,
        funds: &[Coin],
    ) -> ExecuteResult {
        let msg = mock_deposit(address);

        self.execute(app, &msg, sender, funds)
    }

    pub fn execute_withdraw(
        &self,
        app: &mut MockApp,
        sender: Addr,
        asset: impl Into<String>,
        amount: Option<Uint128>,
    ) -> ExecuteResult {
        let msg = mock_withdraw(amount, asset.into());

        self.execute(app, &msg, sender, &[])
    }

    pub fn query_balance(
        &self,
        app: &mut MockApp,
        address: AndrAddr,
        asset: impl Into<String>,
    ) -> Uint128 {
        let msg = mock_balance(address, asset);
        let res: Uint128 = self.query(app, msg);

        res
    }
}
