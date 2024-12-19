use cw_json::JSON;
use cw_storage_plus::Item;

pub const SCHEMA: Item<JSON> = Item::new("schema");
