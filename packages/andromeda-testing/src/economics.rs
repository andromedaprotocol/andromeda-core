use andromeda_economics::mock::*;
use andromeda_std::amp::AndrAddr;
use cosmwasm_std::{Addr, Coin, Uint128};
use cw_multi_test::{App, Executor};

use crate::{mock_ado, mock_contract::ExecuteResult, MockADO, MockContract};

pub struct MockEconomics(Addr);
mock_ado!(MockEconomics);

impl MockEconomics {
    pub fn instantiate(
        app: &mut App,
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
        app: &mut App,
        sender: Addr,
        address: Option<AndrAddr>,
        funds: &[Coin],
    ) -> ExecuteResult {
        let msg = mock_deposit(address);
        let res = self.execute(app, &msg, sender, funds);

        res
    }

    pub fn execute_withdraw(
        &self,
        app: &mut App,
        sender: Addr,
        asset: impl Into<String>,
        amount: Option<Uint128>,
    ) -> ExecuteResult {
        let msg = mock_withdraw(amount, asset.into());
        let res = self.execute(app, &msg, sender, &[]);

        res
    }

    pub fn query_balance(
        &self,
        app: &mut App,
        address: AndrAddr,
        asset: impl Into<String>,
    ) -> Uint128 {
        let msg = mock_balance(address, asset);
        let res: Uint128 = self.query(app, &msg);

        res
    }
}
