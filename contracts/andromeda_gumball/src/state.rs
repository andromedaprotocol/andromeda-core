use andromeda_protocol::gumball::State;
use common::{ado_base::recipient::Recipient, mission::AndrAddress};
use cosmwasm_std::{Coin, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Decided to put the token IDs in a vector
// We'll use the length on the vector to determine the number of available NFTs
pub const LIST: Item<Vec<String>> = Item::new("list of NFTs");
pub const CW721_CONTRACT: Item<String> = Item::new("cw721_contract");
pub const CONFIG: Item<Config> = Item::new("config");
pub const STATE: Item<State> = Item::new("state");
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// The address of the token contract whose tokens are being sold.
    pub token_address: AndrAddress,
}
