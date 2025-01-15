use andromeda_finance::validator_staking::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};
use cw_orch_daemon::{DaemonBase, Wallet};

pub const CONTRACT_ID: &str = "validator-staking";

contract_interface!(
    ValidatorStakingContract,
    CONTRACT_ID,
    "andromeda_validator_staking.wasm"
);

type Chain = DaemonBase<Wallet>;

impl ValidatorStakingContract<Chain> {
    pub fn staked_tokens(&self, validator: Option<Addr>) -> Option<::cosmwasm_std::FullDelegation> {
        let query_msg = QueryMsg::StakedTokens { validator };
        self.query(&query_msg).unwrap()
    }
}
