// use ibc_tests::contract_interface;

use crate::contract_interface;
use andromeda_app::app::{self, AppComponent};
use andromeda_std::ado_base::MigrateMsg;
use andromeda_testing_e2e::mock::MockAndromeda;
use cw_orch::interface;
use cw_orch::prelude::*;
use cw_orch_daemon::{DaemonBase, Wallet};

contract_interface!(
    AppContract,
    andromeda_app_contract,
    app,
    "andromeda_app_contract",
    "app_contract"
);

type Chain = DaemonBase<Wallet>;

impl AppContract<Chain> {
    pub fn init(
        &self,
        andr_os: &MockAndromeda,
        name: impl Into<String>,
        app_components: Vec<AppComponent>,
        owner: Option<String>,
    ) {
        let instantiate_msg = app::InstantiateMsg {
            app_components,
            name: name.into(),
            kernel_address: andr_os.kernel_contract.addr_str().unwrap(),
            owner,
            chain_info: None,
        };
        self.instantiate(&instantiate_msg, None, None).unwrap();
    }
    pub fn query_address_by_component_name(&self, name: impl Into<String>) -> String {
        let query_msg = app::QueryMsg::GetAddress { name: name.into() };
        self.query(&query_msg).unwrap()
    }
}
pub fn prepare(
    daemon: &DaemonBase<Wallet>,
    andr_os: &MockAndromeda,
) -> AppContract<DaemonBase<Wallet>> {
    let app_contract = AppContract::new(daemon.clone());
    app_contract.upload().unwrap();

    let MockAndromeda { adodb_contract, .. } = &andr_os;

    adodb_contract.clone().execute_publish(
        app_contract.code_id().unwrap(),
        "app-contract".to_string(),
        "0.1.0".to_string(),
    );
    app_contract
}
