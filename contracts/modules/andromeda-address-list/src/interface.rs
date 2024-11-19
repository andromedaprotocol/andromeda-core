use andromeda_modules::address_list::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "address_list";

contract_interface!(
    AddressListContract,
    CONTRACT_ID,
    "andromeda_address_list.wasm"
);
