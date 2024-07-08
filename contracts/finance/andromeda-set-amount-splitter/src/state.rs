use andromeda_finance::set_amount_splitter::Splitter;
use cw_storage_plus::Item;

pub const SPLITTER: Item<Splitter> = Item::new("set-amount-splitter");
