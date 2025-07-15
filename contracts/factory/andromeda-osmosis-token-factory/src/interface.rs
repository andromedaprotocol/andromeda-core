use andromeda_std::contract_interface;
pub const CONTRACT_ID: &str = "osmosis-token-factory";


contract_interface!(
    OsmosisTokenFactoryContract,
    CONTRACT_ID,
    "andromeda_osmosis_token_factory.wasm"
);