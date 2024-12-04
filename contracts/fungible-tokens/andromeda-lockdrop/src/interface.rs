use andromeda_fungible_tokens::lockdrop::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "lockdrop";

contract_interface!(LockdropContract, CONTRACT_ID, "andromeda_lockdrop.wasm");
