use common::app::AndrAddress;
use cw_storage_plus::Item;

// The condition ADO we want to send our bool to
pub const TARGET_ADO_ADDRESS: Item<AndrAddress> = Item::new("target_ado_address");
