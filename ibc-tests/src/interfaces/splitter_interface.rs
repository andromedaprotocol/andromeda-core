use crate::contract_interface;
use andromeda_finance::splitter;
use andromeda_std::ado_base::MigrateMsg;
use andromeda_testing_e2e::mock::MockAndromeda;
use cw_orch::interface;
use cw_orch::prelude::*;
use cw_orch_daemon::DaemonBase;
use cw_orch_daemon::Wallet;

contract_interface!(
    SplitterContract,
    andromeda_splitter,
    splitter,
    "andromeda_splitter_contract",
    "splitter"
);

pub fn prepare(
    daemon: &DaemonBase<Wallet>,
    andr_os: &MockAndromeda,
) -> SplitterContract<DaemonBase<Wallet>> {
    let splitter_contract = SplitterContract::new(daemon.clone());
    splitter_contract.upload().unwrap();

    let MockAndromeda { adodb_contract, .. } = &andr_os;

    adodb_contract.clone().execute_publish(
        splitter_contract.code_id().unwrap(),
        "splitter".to_string(),
        "0.1.0".to_string(),
    );
    splitter_contract
}
