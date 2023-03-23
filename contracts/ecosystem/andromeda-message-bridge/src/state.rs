use common::error::ContractError;
use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::Map;
// A mapping of each supported chain to its corresponding channel
pub const CHAIN_CHANNELS: Map<String, String> = Map::new("channels");

pub fn save_channel(storage: &mut dyn Storage, chain: String, channel: String) -> StdResult<()> {
    CHAIN_CHANNELS.save(storage, chain, &channel)
}

pub fn read_channel(storage: &dyn Storage, chain: String) -> StdResult<String> {
    CHAIN_CHANNELS.load(storage, chain)
}

pub fn read_chains(storage: &dyn Storage) -> Result<Vec<String>, ContractError> {
    let chains: Result<Vec<String>, ContractError> = CHAIN_CHANNELS
        .keys(storage, None, None, cosmwasm_std::Order::Descending)
        .map(|x| Ok(x?))
        .collect();
    chains
}
