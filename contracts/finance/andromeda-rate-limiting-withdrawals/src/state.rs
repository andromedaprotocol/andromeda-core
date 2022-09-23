use andromeda_finance::rate_limiting_withdrawals::{AccountDetails, CoinAllowance};
use cw_storage_plus::{Item, Map};

pub const ACCOUNTS: Map<String, AccountDetails> = Map::new("Accounts");
// The allowed coin with its respective withdrawal limit
pub const ALLOWED_COIN: Item<CoinAllowance> = Item::new("allowed coins");
