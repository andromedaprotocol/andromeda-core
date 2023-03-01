use andromeda_automation::condition::LogicGate;
use cw_storage_plus::Item;

// Logic gate setting
pub const LOGIC_GATE: Item<LogicGate> = Item::new("logic_gate");

// List of contracts you want to query results from
pub const EVAL_ADOS: Item<Vec<String>> = Item::new("whitelist");

// Execute ADO's address
pub const EXECUTE_ADO: Item<String> = Item::new("execute_ado");
