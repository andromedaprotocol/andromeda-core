use cosmwasm_std::{Addr, Binary};
use cw_storage_plus::Item;

// The taget ADO we want to send our message to
pub const TARGET_ADO_ADDRESS: Item<String> = Item::new("target_ado_address");

// The condition ADO we want to receive a message from
pub const CONDITION_ADO_ADDRESS: Item<String> = Item::new("condition_ado");

// Task balancer's address
pub const TASK_BALANCER: Item<Addr> = Item::new("task_balancer_address");

// The ExecuteMsg to be sent to the Target ADO
pub const TARGET_MSG: Item<Binary> = Item::new("target_message");
