use andromeda_fungible_tokens::cw20::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};
use cw20::BalanceResponse;
use cw_orch_daemon::{DaemonBase, Wallet};

pub const CONTRACT_ID: &str = "cw20";

contract_interface!(CW20Contract, CONTRACT_ID, "andromeda_cw20.wasm");

type Chain = DaemonBase<Wallet>;

impl CW20Contract<Chain> {
    pub fn balance(&self, address: impl Into<String>) -> BalanceResponse {
        let query_msg = QueryMsg::Balance {
            address: address.into(),
        };
        self.query(&query_msg).unwrap()
    }
}
