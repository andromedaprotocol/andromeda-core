use andromeda_data_storage::form::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "form";

contract_interface!(FormContract, CONTRACT_ID, "andromeda_form.wasm");
