use andromeda_vfs::mock::*;
use cosmwasm_std::Addr;
use cw_multi_test::{App, Executor};

use crate::{mock_ado, mock_contract::ExecuteResult, MockADO, MockContract};

pub struct MockVFS(Addr);
mock_ado!(MockVFS);

impl MockVFS {
    pub fn instantiate(
        app: &mut App,
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
        app: &mut App,
        sender: Addr,
        username: String,
    ) -> ExecuteResult {
        let msg = mock_register_user(username);
        let res = self.execute(app, &msg, sender, &[]);

        res
    }

    pub fn execute_add_path(
        &self,
        app: &mut App,
        sender: Addr,
        name: impl Into<String>,
        address: Addr,
    ) -> ExecuteResult {
        let msg = mock_add_path(name, address);
        let res = self.execute(app, &msg, sender, &[]);

        res
    }

    pub fn query_resolve_path(&self, app: &mut App, path: String) -> String {
        let msg = mock_resolve_path_query(path);
        let res: String = self.query(app, &msg);

        res
    }
}
