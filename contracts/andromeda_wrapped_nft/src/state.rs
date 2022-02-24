use cosmwasm_std::CanonicalAddr;
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONFIG: Item<Config> = Item::new("config");

// key: new_token_id, value: (original_token_id, original_nft_addr)
pub const TOKENIDS: Map<String, (String, String)> = Map::new("token_ids");
pub const CUR_TOKEN_ID: Item<u64> = Item::new("current_token_id");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub name: String,
    pub symbol: String,
    pub factory_addr: CanonicalAddr,
    pub token_addr: CanonicalAddr,
}
