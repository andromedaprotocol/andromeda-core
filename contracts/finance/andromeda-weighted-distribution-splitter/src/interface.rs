use andromeda_finance::weighted_splitter::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "weighted-distribution-splitter";

contract_interface!(
    WeightedDistributionSplitterContract,
    CONTRACT_ID,
    "andromeda_weighted_distribution_splitter.wasm"
);
