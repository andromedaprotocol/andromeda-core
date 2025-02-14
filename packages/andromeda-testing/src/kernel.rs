use crate::mock::MockApp;
use crate::mock_contract::ExecuteResult;

use andromeda_kernel::mock::*;
use andromeda_std::amp::{messages::AMPMsgConfig, AndrAddr};
use andromeda_std::os::kernel::{ExecuteMsg, QueryMsg};
use cosmwasm_std::{Addr, Coin};
use cw_multi_test::Executor;
use serde::Serialize;

use super::{mock_ado, MockADO, MockContract};

pub struct MockKernel(pub Addr);
mock_ado!(MockKernel, ExecuteMsg, QueryMsg);

impl MockKernel {
    pub fn instantiate(
        app: &mut MockApp,
        code_id: u64,
        sender: Addr,
        admin: Option<String>,
        owner: Option<String>,
    ) -> Self {
        let msg = mock_kernel_instantiate_message(owner, "andromeda");
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
        app: &mut MockApp,
        sender: Addr,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> ExecuteResult {
        let msg = mock_upsert_key_address(key, value);

        self.execute(app, &msg, sender, &[])
    }

    pub fn execute_create(
        &self,
        app: &mut MockApp,
        sender: Addr,
        ado_type: impl Into<String>,
        msg: impl Serialize,
        owner: Option<AndrAddr>,
        chain: Option<String>,
    ) -> ExecuteResult {
        let msg = mock_create(ado_type, msg, owner, chain);

        self.execute(app, &msg, sender, &[])
    }

    pub fn execute_send(
        &self,
        app: &mut MockApp,
        sender: Addr,
        recipient: impl Into<String>,
        msg: impl Serialize,
        funds: Vec<Coin>,
        config: Option<AMPMsgConfig>,
    ) -> ExecuteResult {
        let msg = mock_send(recipient, msg, funds.clone(), config);

        self.execute(app, &msg, sender, &funds)
    }

    pub fn query_key_address(&self, app: &MockApp, key: impl Into<String>) -> String {
        let msg = mock_get_key_address(key);

        self.query(app, msg)
    }
}
