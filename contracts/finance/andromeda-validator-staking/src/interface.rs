use andromeda_finance::validator_staking::*;
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "validator-staking";

contract_interface!(
    ValidatorStakingContract,
    CONTRACT_ID,
    "andromeda_validator_staking.wasm"
);
