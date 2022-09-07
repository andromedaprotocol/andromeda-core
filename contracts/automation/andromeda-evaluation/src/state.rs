use common::app::AndrAddress;
use cw_storage_plus::Item;

//
pub const EXECUTE_ADO_ADDRESS: Item<AndrAddress> = Item::new("execute_ado_address");
pub const RESULTS: Item<Vec<bool>> = Item::new("results_from_evaluation_ado");
