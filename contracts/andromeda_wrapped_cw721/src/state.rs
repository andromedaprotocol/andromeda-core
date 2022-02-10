use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const ANDROMEDA_CW721_ADDR: Item<String> = Item::new("andromeda_cw721_addr");
pub const CAN_UNWRAP: Item<bool> = Item::new("can_unwrap");
pub const WRAPPED_TOKENS: Map<&str, WrappedTokenInfo> = Map::new("wrapped_tokens");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WrappedTokenInfo {
    pub wrapped_token_id: String,
    pub original_token_address: String,
}
