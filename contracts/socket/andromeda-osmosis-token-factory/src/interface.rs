use andromeda_socket::osmosis_token_factory::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "osmosis-token-factory";

contract_interface!(
    OsmosisTokenFactoryContract,
    CONTRACT_ID,
    "andromeda_osmosis_token_factory.wasm"
);
