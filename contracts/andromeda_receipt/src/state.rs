use andromeda_protocol::receipt::Receipt;
use cosmwasm_std::{StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map, U128Key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const CONFIG: Item<Config> = Item::new("config");
const RECEIPT: Map<U128Key, Receipt> = Map::new("receipt");
const NUM_RECEIPT: Item<Uint128> = Item::new("num_receipt");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub minter: String,
    pub owner: String,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    CONFIG.save(storage, config)
}

pub fn can_mint_receipt(storage: &dyn Storage, addr: &String) -> StdResult<bool> {
    let config = CONFIG.load(storage)?;
    Ok(addr.eq(&config.owner) || addr.eq(&config.minter))
}

// increase receipt ID
pub fn increment_num_receipt(storage: &mut dyn Storage) -> StdResult<Uint128> {
    let mut receipt_count = NUM_RECEIPT.load(storage).unwrap_or_default();
    receipt_count = receipt_count + Uint128::from(1 as u128);
    NUM_RECEIPT.save(storage, &receipt_count)?;
    Ok(receipt_count)
}

pub fn store_receipt(
    storage: &mut dyn Storage,
    receipt_id: Uint128,
    receipt: &Receipt,
) -> StdResult<()> {
    RECEIPT.save(storage, U128Key::from(receipt_id.u128()), receipt)
}
pub fn read_receipt(storage: &dyn Storage, receipt_id: Uint128) -> StdResult<Receipt> {
    RECEIPT.load(storage, U128Key::from(receipt_id.u128()))
}
