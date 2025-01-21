use andromeda_math::graph::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "graph";

contract_interface!(GraphContract, CONTRACT_ID, "andromeda_graph.wasm");
