use cw_storage_plus::Item;
use andromeda_modules::time_gate::{GateAddresses, GateTime};

pub const GATE_ADDRESSES: Item<GateAddresses> = Item::new("gate_addresses");
pub const GATE_TIME: Item<GateTime> = Item::new("gate_time");
