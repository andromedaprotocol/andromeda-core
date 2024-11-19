use andromeda_modules::rates::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "rates";

contract_interface!(RatesContract, CONTRACT_ID, "andromeda_rates.wasm");
