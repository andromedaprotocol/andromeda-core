use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub mirror_mint_contract: Addr,
    pub mirror_staking_contract: Addr,
    pub mirror_gov_contract: Addr,
    pub mirror_lock_contract: Addr,
    pub mirror_oracle_contract: Addr,
    pub mirror_collateral_oracle_contract: Addr,
}
