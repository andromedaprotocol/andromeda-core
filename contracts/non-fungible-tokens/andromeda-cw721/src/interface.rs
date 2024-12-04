use andromeda_non_fungible_tokens::cw721::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};
use cw_orch_daemon::{DaemonBase, Wallet};

pub const CONTRACT_ID: &str = "cw721";

contract_interface!(CW721Contract, CONTRACT_ID, "andromeda_cw721.wasm");

type Chain = DaemonBase<Wallet>;

impl CW721Contract<Chain> {
    pub fn owner_of(&self, token_id: impl Into<String>) -> cw721::OwnerOfResponse {
        let query_msg = QueryMsg::OwnerOf {
            token_id: token_id.into(),
            include_expired: None,
        };
        self.query(&query_msg).unwrap()
    }
}
