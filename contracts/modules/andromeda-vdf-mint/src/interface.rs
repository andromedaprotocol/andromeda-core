use andromeda_modules::vdf_mint::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "vdf-mint";

contract_interface!(VdfMintContract, CONTRACT_ID, "andromeda_vdf_mint.wasm");
