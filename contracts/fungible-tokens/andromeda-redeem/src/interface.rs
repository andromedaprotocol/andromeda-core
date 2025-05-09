use andromeda_fungible_tokens::redeem::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "redeem";

contract_interface!(RedeemContract, CONTRACT_ID, "andromeda_redeem.wasm");
