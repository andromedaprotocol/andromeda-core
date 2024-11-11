use andromeda_std::{
    ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata, os::economics::*,
};

pub const CONTRACT_ID: &str = "economics";

contract_interface!(EconomicsContract, CONTRACT_ID, "andromeda_economics.wasm");
