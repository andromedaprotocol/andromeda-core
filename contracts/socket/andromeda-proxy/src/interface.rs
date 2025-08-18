use andromeda_socket::proxy::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "osmosis-proxy";

contract_interface!(ProxyContract, CONTRACT_ID, "andromeda_proxy.wasm");
