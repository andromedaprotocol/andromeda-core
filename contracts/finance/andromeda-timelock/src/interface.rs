use andromeda_finance::timelock::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "timelock";

contract_interface!(TimelockContract, CONTRACT_ID, "andromeda_timelock.wasm");
