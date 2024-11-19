use andromeda_app::app::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "app-contract";

contract_interface!(AppContract, CONTRACT_ID, "andromeda_app_contract.wasm");
