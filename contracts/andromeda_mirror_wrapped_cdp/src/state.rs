use cosmwasm_std::CanonicalAddr;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub mirror_mint_contract: CanonicalAddr,
    pub mirror_staking_contract: CanonicalAddr,
    pub mirror_gov_contract: CanonicalAddr,
}
