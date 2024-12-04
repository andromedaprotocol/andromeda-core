use andromeda_modules::schema::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "schema";

contract_interface!(SchemaContract, CONTRACT_ID, "andromeda_schema.wasm");
