use andromeda_math::matrix::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "matrix";

contract_interface!(MatrixContract, CONTRACT_ID, "andromeda_matrix.wasm");
