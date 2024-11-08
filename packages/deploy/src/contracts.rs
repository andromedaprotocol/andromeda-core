use crate::contract_interface;
use andromeda_std::ado_base::MigrateMsg;
use andromeda_std::os::*;
use cw_orch::interface;
use cw_orch::prelude::*;

contract_interface!(
    KernelContract,
    andromeda_kernel,
    kernel,
    "andromeda_kernel",
    "andromeda_kernel.wasm"
);

contract_interface!(
    ADODBContract,
    andromeda_adodb,
    adodb,
    "andromeda_adodb",
    "andromeda_adodb.wasm"
);

contract_interface!(
    VFSContract,
    andromeda_vfs,
    vfs,
    "andromeda_vfs",
    "andromeda_vfs.wasm"
);

contract_interface!(
    EconomicsContract,
    andromeda_economics,
    economics,
    "andromeda_economics",
    "andromeda_economics.wasm"
);

contract_interface!(
    IBCRegistryContract,
    andromeda_ibc_registry,
    ibc_registry,
    "andromeda_ibc_registry",
    "andromeda_ibc_registry.wasm"
);
