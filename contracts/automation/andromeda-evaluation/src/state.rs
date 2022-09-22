use common::app::AndrAddress;
use cw_storage_plus::Item;

// The condition ADO we want to send our bool to
pub const CONDITION_ADO_ADDRESS: Item<AndrAddress> = Item::new("condition_ado_address");

// The address of the ADO we want to query data from
pub const QUERY_ADO_ADDRESS: Item<AndrAddress> = Item::new("query_ado_address");

// Task balancer ADO address
pub const TASK_BALANCER_ADDRESS: Item<AndrAddress> = Item::new("task_balancer_address");
