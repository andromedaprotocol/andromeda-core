use andromeda_finance::set_amount_splitter::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "set-amount-splitter";

contract_interface!(
    SetAmountSplitterContract,
    CONTRACT_ID,
    "andromeda_set_amount_splitter.wasm"
);
