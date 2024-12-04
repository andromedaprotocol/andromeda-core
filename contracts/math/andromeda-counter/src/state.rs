use andromeda_math::counter::CounterRestriction;
use cw_storage_plus::Item;

pub const INITIAL_AMOUNT: Item<u64> = Item::new("initial_amount");
pub const INCREASE_AMOUNT: Item<u64> = Item::new("increase_amount");
pub const DECREASE_AMOUNT: Item<u64> = Item::new("decrease_amount");

pub const CURRENT_AMOUNT: Item<u64> = Item::new("current_amount");

pub const RESTRICTION: Item<CounterRestriction> = Item::new("restriction");
