use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub astroport_factory_contract: Addr,
    pub astroport_router_contract: Addr,
    pub astroport_staking_contract: Addr,
    pub astro_token_contract: Addr,
    pub xastro_token_contract: Addr,
}
