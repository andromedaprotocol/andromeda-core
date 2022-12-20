use andromeda_non_fungible_tokens::cw721_timelock::LockDetails;

use cw_storage_plus::Map;

pub const LOCKED_ITEMS: Map<&str, LockDetails> = Map::new("locked_items");
