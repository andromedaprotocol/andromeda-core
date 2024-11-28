use andromeda_math::shunting::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "shunting";

contract_interface!(ShuntingContract, CONTRACT_ID, "andromeda_shunting.wasm");
