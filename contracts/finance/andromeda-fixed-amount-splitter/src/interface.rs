use andromeda_finance::fixed_amount_splitter::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "fixed-amount-splitter";

contract_interface!(
    FixedAmountSplitterContract,
    CONTRACT_ID,
    "andromeda_fixed_amount_splitter.wasm"
);
