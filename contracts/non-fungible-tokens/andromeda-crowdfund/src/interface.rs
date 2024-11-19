use andromeda_non_fungible_tokens::crowdfund::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "crowdfund";

contract_interface!(CrowdfundContract, CONTRACT_ID, "andromeda_crowdfund.wasm");
