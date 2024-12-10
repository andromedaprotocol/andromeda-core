use andromeda_math::distance::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "distance";

contract_interface!(DistanceContract, CONTRACT_ID, "andromeda_distance.wasm");
