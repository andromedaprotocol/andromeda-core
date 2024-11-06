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
    "andromeda_kernel@1.1.1.wasm"
);

contract_interface!(
    ADODBContract,
    andromeda_adodb,
    adodb,
    "andromeda_adodb",
    "andromeda_adodb@1.1.2.wasm"
);

contract_interface!(
    VFSContract,
    andromeda_vfs,
    vfs,
    "andromeda_vfs",
    "andromeda_vfs@1.1.1.wasm"
);

contract_interface!(
    EconomicsContract,
    andromeda_economics,
    economics,
    "andromeda_economics",
    "andromeda_economics@1.1.1.wasm"
);

contract_interface!(
    IBCRegistryContract,
    andromeda_ibc_registry,
    ibc_registry,
    "andromeda_ibc_registry",
    "andromeda_ibc_registry@1.0.1.wasm"
);
