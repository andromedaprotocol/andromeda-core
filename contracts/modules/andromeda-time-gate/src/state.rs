use andromeda_modules::time_gate::CycleStartTime;
use andromeda_std::amp::AndrAddr;
use cw_storage_plus::Item;

pub const GATE_ADDRESSES: Item<Vec<AndrAddr>> = Item::new("gate_addresses");
pub const CYCLE_START_TIME: Item<CycleStartTime> = Item::new("cycle_start_time");
pub const TIME_INTERVAL: Item<u64> = Item::new("time_interval");
