use andromeda_math::counter::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "counter";

contract_interface!(CounterContract, CONTRACT_ID, "andromeda_counter.wasm");
