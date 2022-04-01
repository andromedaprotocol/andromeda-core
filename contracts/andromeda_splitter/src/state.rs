use andromeda_protocol::splitter::Splitter;
use cw_storage_plus::Item;

pub const SPLITTER: Item<Splitter> = Item::new("splitter");
