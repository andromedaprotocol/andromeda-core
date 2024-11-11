use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata, os::adodb::*};

pub const CONTRACT_ID: &str = "adodb";

contract_interface!(ADODBContract, CONTRACT_ID, "andromeda_adodb.wasm");
