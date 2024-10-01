use crate::contract_interface;
use andromeda_fungible_tokens::cw20;
use andromeda_std::ado_base::MigrateMsg;
use andromeda_testing_e2e::mock::MockAndromeda;
use cw_orch::interface;
use cw_orch::prelude::*;
use cw_orch_daemon::DaemonBase;
use cw_orch_daemon::Wallet;

contract_interface!(
    Cw20Contract,
    andromeda_cw20,
    cw20,
    "andromeda_cw20_contract",
    "cw20"
);

pub fn prepare(
    daemon: &DaemonBase<Wallet>,
    andr_os: &MockAndromeda,
) -> Cw20Contract<DaemonBase<Wallet>> {
    let cw20_contract = Cw20Contract::new(daemon.clone());
    cw20_contract.upload().unwrap();

    let MockAndromeda { adodb_contract, .. } = &andr_os;

    adodb_contract.clone().execute_publish(
        cw20_contract.code_id().unwrap(),
        "cw20".to_string(),
        "0.1.0".to_string(),
    );
    cw20_contract
}
