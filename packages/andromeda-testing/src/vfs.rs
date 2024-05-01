use crate::{mock::MockApp, mock_ado, mock_contract::ExecuteResult, MockADO, MockContract};
use andromeda_std::os::vfs::{ExecuteMsg, QueryMsg};
use andromeda_vfs::mock::*;
use cosmwasm_std::Addr;
use cw_multi_test::Executor;

pub struct MockVFS(Addr);
mock_ado!(MockVFS, ExecuteMsg, QueryMsg);

impl MockVFS {
    pub fn instantiate(
        app: &mut MockApp,
        code_id: u64,
        sender: Addr,
        owner: Option<String>,
        kernel_address: String,
    ) -> Self {
        let msg = mock_vfs_instantiate_message(kernel_address, owner);
        let res = app.instantiate_contract(
            code_id,
            sender.clone(),
            &msg,
            &[],
            "VFS",
            Some(sender.to_string()),
        );

        Self(res.unwrap())
    }

    pub fn execute_register_user(
        &self,
        app: &mut MockApp,
        sender: Addr,
        username: String,
    ) -> ExecuteResult {
        let msg = mock_register_user(username);

        self.execute(app, &msg, sender, &[])
    }

    pub fn execute_add_path(
        &self,
        app: &mut MockApp,
        sender: Addr,
        name: impl Into<String>,
        address: Addr,
    ) -> ExecuteResult {
        let msg = mock_add_path(name, address);

        self.execute(app, &msg, sender, &[])
    }

    pub fn query_resolve_path(&self, app: &mut MockApp, path: String) -> Addr {
        let msg = mock_resolve_path_query(path);
        let res: Addr = self.query(app, msg);

        res
    }
}
