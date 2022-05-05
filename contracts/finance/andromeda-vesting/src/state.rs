use common::{ado_base::recipient::Recipient, error::ContractError, withdraw::WithdrawalType};
use cosmwasm_std::{Order, Storage, Uint128};
use cw0::Expiration;
use cw_storage_plus::{Bound, Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Maps Id to Batch.
pub const BATCHES: Map<&str, Batch> = Map::new("batches");

/// The config.
pub const CONFIG: Item<Config> = Item::new("config");

/// The next ID to use for a newly added batch.
const NEXT_ID: Item<Uint128> = Item::new("next_id");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// The recipient of each batch.
    pub recipient: Recipient,
    /// Whether or not multiple batches are supported.
    pub is_multi_batch_enabled: bool,
    /// The denom of the coin being vested.
    pub denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Batch {
    /// The amount of tokens in the batch
    pub amount: Uint128,
    /// The amount of tokens that have been claimed.
    pub amount_claimed: Uint128,
    /// When the lockup ends.
    pub lockup_end: Expiration,
    /// How often releases occur.
    pub release_unit: u64,
    /// Specifies how much is to be released after each `release_unit`. If
    /// it is a percentage, it would be the percentage of the original amount.
    pub release_amount: WithdrawalType,
    /// The time at which the last claim took place in seconds.
    pub last_claim_time: u64,
}

pub(crate) fn save_new_batch(storage: &mut dyn Storage, batch: Batch) -> Result<(), ContractError> {
    let next_id = NEXT_ID
        .may_load(storage)?
        .unwrap_or_else(|| Uint128::new(1));

    BATCHES.save(storage, &next_id.to_string(), &batch)?;
    NEXT_ID.save(storage, &(next_id + Uint128::new(1)))?;

    Ok(())
}

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;
pub(crate) fn get_batch_ids(
    storage: &dyn Storage,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<String>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let batch_ids: Result<Vec<String>, ContractError> = BATCHES
        .keys(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|k| Ok(String::from_utf8(k)?))
        .collect();

    batch_ids
}
