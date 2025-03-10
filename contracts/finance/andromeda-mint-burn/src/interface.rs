use andromeda_finance::mint_burn::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "mint-burn";

contract_interface!(MintBurnContract, CONTRACT_ID, "andromeda_mint_burn.wasm");
