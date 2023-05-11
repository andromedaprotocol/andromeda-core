use andromeda_modules::rates::RateInfo;
use cosmwasm_schema::cw_serde;
use cw_storage_plus::Item;

pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct Config {
    pub rates: Vec<RateInfo>,
}
