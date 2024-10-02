use crate::contract_interface;
use andromeda_fungible_tokens::cw20 as andr_cw20;
use andromeda_std::ado_base::MigrateMsg;
use andromeda_std::amp::AndrAddr;
use andromeda_testing_e2e::mock::MockAndromeda;
use cosmwasm_std::to_json_binary;
use cosmwasm_std::Uint128;
use cw20::BalanceResponse;
use cw_orch::interface;
use cw_orch::prelude::*;
use cw_orch_daemon::DaemonBase;
use cw_orch_daemon::Wallet;
use serde::Serialize;

contract_interface!(
    Cw20Contract,
    andromeda_cw20,
    andr_cw20,
    "andromeda_cw20_contract",
    "cw20"
);

type Chain = DaemonBase<Wallet>;

impl Cw20Contract<Chain> {
    pub fn query_balance(&self, address: impl Into<String>) -> BalanceResponse {
        let query_msg = andr_cw20::QueryMsg::Balance { address: address.into() };
        self.query(&query_msg).unwrap()
    }

    pub fn execute_send(&self, contract: impl Into<String>, amount: Uint128, msg: &impl Serialize) {
        let execute_msg = andr_cw20::ExecuteMsg::Send {
            contract: AndrAddr::from_string(contract.into()), amount, msg: to_json_binary(msg).unwrap()
        };
        self.execute(&execute_msg, None).unwrap();
    }
}


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
