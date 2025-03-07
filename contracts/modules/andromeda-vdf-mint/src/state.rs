use andromeda_std::amp::AndrAddr;
use cosmwasm_std::{Addr, Uint64};
use cw_storage_plus::Item;

pub const LAST_MINT_TIMESTAMP: Item<Uint64> = Item::new("last_mint_timestamp");
pub const ACTORS: Item<Vec<Addr>> = Item::new("actors");
pub const CW721_ADDRESS: Item<AndrAddr> = Item::new("cw721_address");
pub const MINT_COOLDOWN_MINUTES: Item<Uint64> = Item::new("mint_cooldown_minutes");
