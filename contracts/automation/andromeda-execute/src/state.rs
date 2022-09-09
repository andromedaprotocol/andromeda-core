use common::app::AndrAddress;
use cw_storage_plus::Item;

// The taget ADO we want to send our message to
pub const TARGET_ADO_ADDRESS: Item<AndrAddress> = Item::new("target_ado_address");

// The condition ADO we want to receive a message from
pub const CONDITION_ADO_ADDRESS: Item<AndrAddress> = Item::new("condition_ado");
