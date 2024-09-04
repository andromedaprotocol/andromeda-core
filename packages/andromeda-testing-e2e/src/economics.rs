use crate::contract_interface;
use andromeda_std::ado_base::MigrateMsg;
use cw_orch::interface;
use cw_orch::prelude::*;

use andromeda_std::os::economics;
use cw_orch_daemon::DaemonBase;
use cw_orch_daemon::Wallet;

contract_interface!(
    EconomicsContract,
    andromeda_economics,
    economics,
    "economics_contract",
    "economics"
);
impl EconomicsContract<DaemonBase<Wallet>> {
    pub fn init(self, kernel_address: String) {
        let msg = economics::InstantiateMsg {
            kernel_address,
            owner: None,
        };
        self.instantiate(&msg, None, None).unwrap();
    }
}
