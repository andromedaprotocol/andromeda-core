use andromeda_fungible_tokens::cw20::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "cw20";

contract_interface!(CW20Contract, CONTRACT_ID, "andromeda_cw20.wasm");
