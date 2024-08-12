use crate::contract_interface;
use cw_orch::interface;
use cw_orch::prelude::*;
use andromeda_std::ado_base::MigrateMsg;

use andromeda_std::os::adodb;
use cw_orch_daemon::DaemonBase;
use cw_orch_daemon::Wallet;

contract_interface!(
    AdodbContract,
    andromeda_adodb,
    adodb,
    "adodb_contract",
    "adodb"
);
impl AdodbContract<DaemonBase<Wallet>> {
    pub fn init(self, kernel_address: String) {
        let msg = adodb::InstantiateMsg {
            kernel_address,
            owner: None,
        };
        self.instantiate(&msg, None, None).unwrap();
    }

    pub fn execute_publish(self, code_id: u64, ado_type: String, version: String) {
        self.execute(
            &adodb::ExecuteMsg::Publish {
                code_id: code_id,
                ado_type,
                action_fees: None,
                version,
                publisher: None,
            },
            None,
        )
        .unwrap();

    }
}