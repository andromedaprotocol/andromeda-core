use andromeda_automation::evaluation::Operators;
use common::app::AndrAddress;
use cosmwasm_std::Uint128;
use cw_storage_plus::Item;

// The condition ADO we want to send our bool to
pub const CONDITION_ADO_ADDRESS: Item<AndrAddress> = Item::new("condition_ado_address");

// The address of the ADO we want to query data from
pub const ORACLE_ADO_ADDRESS: Item<AndrAddress> = Item::new("query_ado_address");

// Task balancer ADO address
pub const TASK_BALANCER_ADDRESS: Item<AndrAddress> = Item::new("task_balancer_address");

// The value we want to compare with the oracle's
pub const VALUE: Item<Option<Uint128>> = Item::new("stored_value");

// Sets the way we want to compare the Oracle's value to the other's. Either greater, less ...
pub const OPERATION: Item<Operators> = Item::new("operation");
