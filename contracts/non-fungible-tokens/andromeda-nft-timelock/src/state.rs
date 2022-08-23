use cw721::Expiration;
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const LOCKED_ITEMS: Map<&str, LockDetails> = Map::new("locked_items");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct LockDetails {
    pub recipient: String,
    pub expiration: Expiration,
    pub nft_id: String,
    pub nft_contract: String,
}
