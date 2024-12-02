use andromeda_non_fungible_tokens::marketplace::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "marketplace";

contract_interface!(
    MarketplaceContract,
    CONTRACT_ID,
    "andromeda_marketplace.wasm"
);
