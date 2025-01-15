use andromeda_finance::vesting::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "vesting";

contract_interface!(VestingContract, CONTRACT_ID, "andromeda_vesting.wasm");
