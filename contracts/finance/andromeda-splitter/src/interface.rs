use andromeda_finance::splitter::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "splitter";

contract_interface!(SplitterContract, CONTRACT_ID, "andromeda_splitter.wasm");
