use andromeda_automation::execute::Increment;
use common::app::AndrAddress;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

// The taget ADO we want to send our message to
pub const TARGET_ADO_ADDRESS: Item<AndrAddress> = Item::new("target_ado_address");

// The condition ADO we want to receive a message from
pub const CONDITION_ADO_ADDRESS: Item<AndrAddress> = Item::new("condition_ado");

// Placeholder for the current demo
pub const INCREMENT_MESSAGE: Item<Increment> = Item::new("Increment");

pub const TASK_BALANCER: Item<Addr> = Item::new("task_balancer_address");
