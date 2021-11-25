use cw_storage_plus::{Item};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use andromeda_protocol::modules::Rate;

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: String,
    pub rates: Vec<Rate>,
    pub is_additive: bool,
    pub description: String,
}
