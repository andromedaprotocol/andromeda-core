use crate::contract_interface;
use andromeda_non_fungible_tokens::cw721 as andr_cw721;
use andromeda_std::ado_base::MigrateMsg;
use andromeda_testing_e2e::mock::MockAndromeda;
use cw721::OwnerOfResponse;
use cw_orch::interface;
use cw_orch::prelude::*;
use cw_orch_daemon::DaemonBase;
use cw_orch_daemon::Wallet;

contract_interface!(
    Cw721Contract,
    andromeda_cw721,
    andr_cw721,
    "andromeda_cw721_contract",
    "cw721"
);
type Chain = DaemonBase<Wallet>;

impl Cw721Contract<Chain> {
    pub fn query_owner_of(&self, token_id: impl Into<String>) -> OwnerOfResponse {
        let query_msg = andr_cw721::QueryMsg::OwnerOf {
            token_id: token_id.into(),
            include_expired: None,
        };
        self.query(&query_msg).unwrap()
    }
}

pub fn prepare(
    daemon: &DaemonBase<Wallet>,
    andr_os: &MockAndromeda,
) -> Cw721Contract<DaemonBase<Wallet>> {
    let cw721_contract = Cw721Contract::new(daemon.clone());
    cw721_contract.upload().unwrap();

    let MockAndromeda { adodb_contract, .. } = &andr_os;

    adodb_contract.clone().execute_publish(
        cw721_contract.code_id().unwrap(),
        "cw721".to_string(),
        "0.1.0".to_string(),
    );
    cw721_contract
}
