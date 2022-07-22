use andromeda_finance::rate_limiting_withdrawals::AccountDetails;
use cosmwasm_std::Uint128;
use cw_storage_plus::{Item, Map};

pub const ACCOUNTS: Map<String, AccountDetails> = Map::new("Accounts");
pub const MINIMUM_WITHDRAWAL_FREQUENCY: Item<u64> = Item::new("minimum withdrawal frequency");
// A map of allowed coins with their respective withdrawal limit
pub const ALLOWED_COINS: Map<&str, Uint128> = Map::new("allowed coins");
