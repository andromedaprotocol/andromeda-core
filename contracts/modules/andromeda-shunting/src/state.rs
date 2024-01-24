use andromeda_modules::shunting::ShuntingObject;
use cw_storage_plus::Item;

pub const SHUNTING: Item<ShuntingObject> = Item::new("shunting");
