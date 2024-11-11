use andromeda_non_fungible_tokens::cw721::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "cw721";

contract_interface!(CW721Contract, CONTRACT_ID, "andromeda_cw721.wasm");
