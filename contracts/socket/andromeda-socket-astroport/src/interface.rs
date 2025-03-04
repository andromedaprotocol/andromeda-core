use andromeda_socket::astroport::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "socket_astroport";

contract_interface!(
    SocketAstroportContract,
    CONTRACT_ID,
    "andromeda_socket_astroport.wasm"
);
