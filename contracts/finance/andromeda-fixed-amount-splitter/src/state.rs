use andromeda_finance::fixed_amount_splitter::Splitter;
use cw_storage_plus::Item;

pub const SPLITTER: Item<Splitter> = Item::new("fixed-amount-splitter");
