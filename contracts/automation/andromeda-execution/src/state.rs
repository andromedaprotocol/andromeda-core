use andromeda_automation::execution::LogicGate;
use cw_storage_plus::Item;

// Logic gate setting
pub const LOGIC_GATE: Item<LogicGate> = Item::new("logic_gate");
