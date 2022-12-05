use cosmwasm_std::Addr;
use cw_storage_plus::Item;

// The currently stored processes
pub const PROCESSES: Item<Vec<Addr>> = Item::new("processes");

// The task balancer's address
pub const TASK_BALANCER: Item<Addr> = Item::new("task_balancer");

// Maximum number of processes that can be stored
pub const MAX_PROCESSES: Item<u64> = Item::new("limit_of_processes");
