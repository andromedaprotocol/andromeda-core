use andromeda_math::date_time::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "date-time";

contract_interface!(DateTimeContract, CONTRACT_ID, "andromeda_date_time.wasm");
