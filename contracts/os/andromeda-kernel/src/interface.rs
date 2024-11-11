use andromeda_std::{
    ado_base::MigrateMsg,
    contract_interface,
    deploy::ADOMetadata,
    os::kernel::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

pub const CONTRACT_ID: &str = "kernel";

contract_interface!(KernelContract, CONTRACT_ID, "andromeda_kernel.wasm");
