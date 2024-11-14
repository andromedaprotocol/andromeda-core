use andromeda_data_storage::string::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "string-storage";

contract_interface!(
    StringStorageContract,
    CONTRACT_ID,
    "andromeda_string_storage.wasm"
);
