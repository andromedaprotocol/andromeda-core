use cosmwasm_std::{Storage, Uint128, StdResult};
use cw_storage_plus::{Item, U128Key, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use andromeda_protocol::receipt::Receipt;

const CONFIG: Item<Config> = Item::new("config");
const RECEIPT: Map<U128Key, Receipt> = Map::new("receipt");
const NUM_RECEIPT: Item<Uint128> = Item::new("num_receipt");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: String,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    CONFIG.save(storage, config)
}

// increase receipt ID
pub fn increment_num_receipt(storage: &mut dyn Storage) -> StdResult<Uint128> {
    let mut receipt_count = NUM_RECEIPT.load(storage).unwrap_or_default();
    receipt_count = receipt_count + Uint128::from(1 as u128);
    NUM_RECEIPT.save(storage, &receipt_count)?;
    Ok(receipt_count)
}

pub fn store_receipt(storage: &mut dyn Storage, receipt_id: Uint128, receipt: &Receipt)->StdResult<()>{
    RECEIPT.save(storage, U128Key::from(receipt_id.u128()), receipt)
}
pub fn read_receipt(storage: &dyn Storage, receipt_id:Uint128)->StdResult<Receipt>{
    RECEIPT.load(storage, U128Key::from(receipt_id.u128()))
}