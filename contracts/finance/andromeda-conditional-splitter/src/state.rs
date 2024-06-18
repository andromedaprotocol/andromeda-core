use andromeda_finance::conditional_splitter::ConditionalSplitter;
use cw_storage_plus::Item;

pub const CONDITIONAL_SPLITTER: Item<ConditionalSplitter> = Item::new("conditional_splitter");
