use andromeda_protocol::communication::Recipient;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONFIG: Item<Config> = Item::new("config");
pub const POSITION: Map<&str, Position> = Map::new("position");
pub const PREV_AUST_BALANCE: Item<Uint128> = Item::new("prev_aust_balance");
pub const PREV_UUSD_BALANCE: Item<Uint128> = Item::new("prev_uusd_balance");
pub const RECIPIENT_ADDR: Item<String> = Item::new("recipient_addr");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub anchor_market: Addr,
    pub aust_token: Addr,
    pub anchor_bluna_hub: Addr,
    pub anchor_bluna_custody: Addr,
    pub anchor_overseer: Addr,
    pub bluna_token: Addr,
    pub anchor_oracle: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Position {
    pub recipient: Recipient,
    pub aust_amount: Uint128,
}
