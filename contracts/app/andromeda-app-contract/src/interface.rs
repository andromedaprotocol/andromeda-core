use andromeda_app::app::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};
use cw_orch_daemon::{DaemonBase, Wallet};

pub const CONTRACT_ID: &str = "app-contract";

contract_interface!(AppContract, CONTRACT_ID, "andromeda_app_contract.wasm");

type Chain = DaemonBase<Wallet>;

impl AppContract<Chain> {
    pub fn get_address(&self, name: impl Into<String>) -> String {
        let query_msg = QueryMsg::GetAddress { name: name.into() };
        self.query(&query_msg).unwrap()
    }
}
