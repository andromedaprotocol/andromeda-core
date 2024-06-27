use cw_storage_plus::Item;

pub const EXPRESSIONS: Item<Vec<String>> = Item::new("expressions");
