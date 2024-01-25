use crate::mock_contract::ExecuteResult;

use super::mock_contract::{MockADO, MockContract};
use andromeda_kernel::mock::*;
use andromeda_std::amp::{messages::AMPMsgConfig, AndrAddr};
use cosmwasm_std::{Addr, Coin};
use cw_multi_test::{App, Executor};
use serde::Serialize;

use super::mock_ado;

pub struct MockKernel(pub Addr);
mock_ado!(MockKernel);

impl MockKernel {
    pub fn instantiate(
        app: &mut App,
        code_id: u64,
        sender: Addr,
        admin: Option<String>,
        owner: Option<String>,
    ) -> Self {
        let msg = mock_kernel_instantiate_message(owner);
        let res = app.instantiate_contract(
            code_id,
            sender.clone(),
            &msg,
            &[],
            "Andromeda Kernel",
            Some(admin.unwrap_or(sender.into())),
        );

        Self(res.unwrap())
    }

    pub fn execute_store_key_address(
        &self,
        app: &mut App,
        sender: Addr,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> ExecuteResult {
        let msg = mock_upsert_key_address(key, value);
        let res = self.execute(app, &msg, sender, &[]);

        res
    }

    pub fn execute_create(
        &self,
        app: &mut App,
        sender: Addr,
        ado_type: impl Into<String>,
        msg: impl Serialize,
        owner: Option<AndrAddr>,
        chain: Option<String>,
    ) -> ExecuteResult {
        let msg = mock_create(ado_type, msg, owner, chain);
        let res = self.execute(app, &msg, sender, &[]);

        res
    }

    pub fn execute_send(
        &self,
        app: &mut App,
        sender: Addr,
        recipient: impl Into<String>,
        msg: impl Serialize,
        funds: Vec<Coin>,
        config: Option<AMPMsgConfig>,
    ) -> ExecuteResult {
        let msg = mock_send(recipient, msg, funds.clone(), config);
        let res = self.execute(app, &msg, sender, &funds);

        res
    }

    pub fn query_key_address(&self, app: &App, key: impl Into<String>) -> String {
        let msg = mock_get_key_address(key);
        let res = self.query(app, &msg);

        res
    }
}
