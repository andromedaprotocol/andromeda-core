use andromeda_automation::evaluation::Operators;
use common::app::AndrAddress;
use cw_storage_plus::Item;

// The condition ADO we want to send our bool to
pub const EXECUTE_ADO_ADDRESS: Item<AndrAddress> = Item::new("execute_ado_address");

// The operation we want to yield true
pub const OPERATION: Item<Operators> = Item::new("desired operation");
