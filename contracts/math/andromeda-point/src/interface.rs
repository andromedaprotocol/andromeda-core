use andromeda_math::point::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "point";

contract_interface!(PointContract, CONTRACT_ID, "andromeda_point.wasm");
