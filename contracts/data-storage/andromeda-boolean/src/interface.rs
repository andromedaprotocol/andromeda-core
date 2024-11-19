use andromeda_data_storage::boolean::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "boolean";

contract_interface!(BooleanContract, CONTRACT_ID, "andromeda_boolean.wasm");
