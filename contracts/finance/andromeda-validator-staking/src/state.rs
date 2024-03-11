use cw_storage_plus::{Deque, Item};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin, Timestamp};

pub const DEFAULT_VALIDATOR: Item<Addr> = Item::new("default_validator");

pub const UNSTAKING_QUEUE: Deque<Unstaking> = Deque::new("unstaking_queue");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Unstaking {
    pub fund: Coin,
    pub payout_at: Timestamp,
}
