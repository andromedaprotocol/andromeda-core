use andromeda_data_storage::primitive::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "primitive";

contract_interface!(PrimitiveContract, CONTRACT_ID, "andromeda_primitive.wasm");
