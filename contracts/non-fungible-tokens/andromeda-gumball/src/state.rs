use common::{ado_base::recipient::Recipient, app::AndrAddress};
use cosmwasm_std::{Coin, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Decided to put the token IDs in a vector
// We'll use the length on the vector to determine the number of available NFTs
pub const REQUIRED_COIN: Item<String> = Item::new("required_coin");
pub const LIST: Item<Vec<String>> = Item::new("list of NFTs");
pub const CW721_CONTRACT: Item<AndrAddress> = Item::new("cw721_contract");
pub const RANDOMNESS_PROVIDER: Item<String> = Item::new("source of randomness");
pub const STATE: Item<State> = Item::new("state");
pub const STATUS: Item<bool> = Item::new("status");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    /// The price of each token.
    pub price: Coin,
    /// The max number of tokens allowed per wallet.
    pub max_amount_per_wallet: Uint128,
    /// The recipient of the funds upon a sale.
    /// Most likely the contract, but could also be the splitter contract for example.
    pub recipient: Recipient,
}
