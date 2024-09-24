use crate::contract_interface;
use andromeda_std::ado_base::MigrateMsg;
use cw_orch::interface;
use cw_orch::prelude::*;

use andromeda_std::os::kernel;
use cw_orch_daemon::DaemonBase;
use cw_orch_daemon::Wallet;

contract_interface!(
    KernelContract,
    andromeda_kernel,
    kernel,
    "kernel_contract",
    "kernel"
);

impl KernelContract<DaemonBase<Wallet>> {
    pub fn init(self, chain_name: String) {
        let msg = kernel::InstantiateMsg {
            chain_name,
            owner: None,
        };
        self.instantiate(&msg, None, None).unwrap();
    }

    pub fn execute_store_key_address(self, key: String, value: String) {
        self.execute(&kernel::ExecuteMsg::UpsertKeyAddress { key, value }, None)
            .unwrap();
    }
}
