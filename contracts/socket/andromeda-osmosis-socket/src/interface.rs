use andromeda_socket::osmosis::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "socket-osmosis";

contract_interface!(
    SocketOsmosisContract,
    CONTRACT_ID,
    "andromeda_osmosis_socket.wasm"
);
