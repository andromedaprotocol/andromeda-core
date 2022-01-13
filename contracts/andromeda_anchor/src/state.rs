use cosmwasm_std::{CanonicalAddr, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};


pub const CONFIG: Item<Config> = Item::new("config");
pub const KEY_POSITION_IDX: Item<Uint128> = Item::new("position_idx");
pub const POSITION: Map<&[u8], Position> = Map::new("position");
pub const PREV_AUST_BALANCE: Item<Uint128> = Item::new("prev_aust_balance");
pub const TEMP_BALANCE: Item<Uint128> = Item::new("temp_balance");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config{
    pub anchor_mint: CanonicalAddr,
    pub anchor_token: CanonicalAddr,
    pub stable_denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Position {
    pub idx: Uint128,
    pub owner: CanonicalAddr,
    pub deposit_amount: Uint128,
    pub aust_amount: Uint128,
}