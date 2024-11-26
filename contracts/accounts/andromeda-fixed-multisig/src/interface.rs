use andromeda_accounts::fixed_multisig::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "fixed-multisig";

contract_interface!(
    FixedMultisigContract,
    CONTRACT_ID,
    "andromeda_fixed_multisig.wasm"
);
