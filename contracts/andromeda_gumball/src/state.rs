use andromeda_protocol::gumball::State;
use common::{ado_base::recipient::Recipient, mission::AndrAddress};
use cosmwasm_std::{Coin, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Decided to put the token IDs in a vector
// We'll use the length on the vector to determine the number of available NFTs
pub const LIST: Item<Vec<String>> = Item::new("list of NFTs");
pub const CW721_CONTRACT: Item<AndrAddress> = Item::new("cw721_contract");
pub const STATE: Item<State> = Item::new("state");
pub const STATUS: Item<bool> = Item::new("status");
