use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Uint128};
use cw_asset::AssetInfo;
use cw_storage_plus::{Item, Map};
use cw_utils::Expiration;

#[cw_serde]
pub struct Config {
    pub asset_info: AssetInfo,
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const LATEST_STAGE_KEY: &str = "stage";
pub const LATEST_STAGE: Item<u8> = Item::new(LATEST_STAGE_KEY);

pub const STAGE_EXPIRATION_KEY: &str = "stage_exp";
pub const STAGE_EXPIRATION: Map<u8, Expiration> = Map::new(STAGE_EXPIRATION_KEY);

pub const STAGE_AMOUNT_KEY: &str = "stage_amount";
pub const STAGE_AMOUNT: Map<u8, Uint128> = Map::new(STAGE_AMOUNT_KEY);

pub const STAGE_AMOUNT_CLAIMED_KEY: &str = "stage_claimed_amount";
pub const STAGE_AMOUNT_CLAIMED: Map<u8, Uint128> = Map::new(STAGE_AMOUNT_CLAIMED_KEY);

pub const MERKLE_ROOT_PREFIX: &str = "merkle_root";
pub const MERKLE_ROOT: Map<u8, String> = Map::new(MERKLE_ROOT_PREFIX);

pub const CLAIM_PREFIX: &str = "claim";
pub const CLAIM: Map<(&Addr, u8), bool> = Map::new(CLAIM_PREFIX);
