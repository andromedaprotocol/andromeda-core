use cosmwasm_std::Addr;
use cw_storage_plus::Item;

// The taget ADO we want to send our message to
pub const PROCESSES: Item<Vec<Addr>> = Item::new("processes");

// The condition ADO we want to receive a message from
pub const TASK_BALANCER: Item<Addr> = Item::new("task_balancer");

pub const MAX_PROCESSES: Item<u64> = Item::new("liit_of_processes");
