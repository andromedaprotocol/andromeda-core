use common::ado_base::recipient::Recipient;
use cosmwasm_std::Uint128;
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const POSITION: Map<&str, Position> = Map::new("position");
pub const PREV_AUST_BALANCE: Item<Uint128> = Item::new("prev_aust_balance");
pub const PREV_UUSD_BALANCE: Item<Uint128> = Item::new("prev_uusd_balance");
pub const RECIPIENT_ADDR: Item<String> = Item::new("recipient_addr");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Position {
    pub recipient: Recipient,
    pub aust_amount: Uint128,
}
