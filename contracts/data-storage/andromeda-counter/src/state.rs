use andromeda_data_storage::counter::CounterRestriction;
use cw_storage_plus::Item;

pub const DEFAULT_INITIAL_AMOUNT: u64 = 0;
pub const DEFAULT_INCREASE_AMOUNT: u64 = 1;
pub const DEFAULT_DECREASE_AMOUNT: u64 = 1;

pub const INITIAL_AMOUNT: Item<u64> = Item::new("initial_amount");
pub const INCREASE_AMOUNT: Item<u64> = Item::new("increase_amount");
pub const DECREASE_AMOUNT: Item<u64> = Item::new("decrease_amount");

pub const CURRENT_AMOUNT: Item<u64> = Item::new("current_amount");

pub const RESTRICTION: Item<CounterRestriction> = Item::new("restriction");
