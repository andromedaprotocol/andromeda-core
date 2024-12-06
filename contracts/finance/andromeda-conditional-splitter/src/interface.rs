use andromeda_finance::conditional_splitter::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "conditional-splitter";

contract_interface!(
    ConditionalSplitterContract,
    CONTRACT_ID,
    "andromeda_conditional_splitter.wasm"
);
