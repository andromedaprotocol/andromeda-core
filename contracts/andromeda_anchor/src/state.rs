use andromeda_protocol::anchor::Position;
use cosmwasm_std::{Addr, CanonicalAddr, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, U128Key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONFIG: Item<Config> = Item::new("config");
pub const KEY_POSITION_IDX: Item<Uint128> = Item::new("position_idx");
pub const PREV_AUST_BALANCE: Item<Uint128> = Item::new("prev_aust_balance");
pub const TEMP_BALANCE: Item<Uint128> = Item::new("temp_balance");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub anchor_mint: CanonicalAddr,
    pub anchor_token: CanonicalAddr,
    pub stable_denom: String,
}

pub struct PositionIndexes<'a> {
    pub owner: MultiIndex<'a, (Addr, U128Key), Position>,
}

impl<'a> IndexList<Position> for PositionIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Position>> + '_> {
        let v: Vec<&dyn Index<Position>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

pub fn positions<'a>() -> IndexedMap<'a, U128Key, Position, PositionIndexes<'a>> {
    let indexes = PositionIndexes {
        owner: MultiIndex::new(
            |p, k| (p.owner.clone(), k.into()),
            "ownership",
            "positions_owner",
        ),
    };
    IndexedMap::new("ownership", indexes)
}

