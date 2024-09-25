use crate::contract_interface;
use andromeda_std::ado_base::MigrateMsg;
use cw_orch::interface;
use cw_orch::prelude::*;

use andromeda_std::os::vfs;
use cw_orch_daemon::DaemonBase;
use cw_orch_daemon::Wallet;

contract_interface!(VfsContract, andromeda_vfs, vfs, "vfs_contract", "vfs");
impl VfsContract<DaemonBase<Wallet>> {
    pub fn init(self, kernel_address: String) {
        let msg = vfs::InstantiateMsg {
            kernel_address,
            owner: None,
        };
        self.instantiate(&msg, None, None).unwrap();
    }
}
