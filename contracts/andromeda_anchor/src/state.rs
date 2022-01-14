use cosmwasm_std::{Addr, CanonicalAddr, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex, U128Key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONFIG: Item<Config> = Item::new("config");
pub const KEY_POSITION_IDX: Item<Uint128> = Item::new("position_idx");
pub const POSITION: Map<&[u8], Position> = Map::new("position");
//pub const POSITION_IDXS: Map<&str, Vec<Uint128>> = Map::new("position_idxs");
pub const PREV_AUST_BALANCE: Item<Uint128> = Item::new("prev_aust_balance");
pub const TEMP_BALANCE: Item<Uint128> = Item::new("temp_balance");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub anchor_mint: CanonicalAddr,
    pub anchor_token: CanonicalAddr,
    pub stable_denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Position {
    pub idx: Uint128,
    pub owner: Addr,
    pub deposit_amount: Uint128,
    pub aust_amount: Uint128,
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

