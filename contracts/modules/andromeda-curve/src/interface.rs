use andromeda_modules::curve::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "curve";

contract_interface!(CurveContract, CONTRACT_ID, "andromeda_curve.wasm");
