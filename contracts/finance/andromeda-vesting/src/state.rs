use common::{
    ado_base::recipient::Recipient, error::ContractError, require, withdraw::WithdrawalType,
};
use cosmwasm_std::{Order, Storage, Uint128};
use cw_storage_plus::{
    Bound, Index, IndexList, IndexedMap, Item, MultiIndex, PrimaryKey, U64Key, U8Key,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The config.
pub const CONFIG: Item<Config> = Item::new("config");

/// The next ID to use for a newly added batch.
pub const NEXT_ID: Item<u64> = Item::new("next_id");

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
    pub lockup_end: u64,
    /// How often releases occur.
    pub release_unit: u64,
    /// Specifies how much is to be released after each `release_unit`. If
    /// it is a percentage, it would be the percentage of the original amount.
    pub release_amount: WithdrawalType,
    /// The time at which the last claim took place in seconds.
    pub last_claim_time: u64,
}

// Inspired by https://docs.cosmwasm.com/tutorials/storage/indexes/#storage-plus-indexing
// We need a secondary index for batches, such that we can look up batches that
// still have funds, ordered by expiration (ascending) from now.
// Index: (U8Key/bool: batch_fully_claimed, U64Key: lockup_end) -> U64Key: pk
pub struct BatchIndexes<'a> {
    pub claim_time: MultiIndex<'a, (U8Key, U64Key, U64Key), Batch>,
}

impl<'a> IndexList<Batch> for BatchIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Batch>> + '_> {
        let v: Vec<&dyn Index<Batch>> = vec![&self.claim_time];
        Box::new(v.into_iter())
    }
}

pub fn batches<'a>() -> IndexedMap<'a, U64Key, Batch, BatchIndexes<'a>> {
    let indexes = BatchIndexes {
        claim_time: MultiIndex::new(
            |b: &Batch, pk: Vec<u8>| {
                let all_claimed = b.amount - b.amount_claimed == Uint128::zero();
                // Allows us to skip batches that have been already fully claimed.
                let all_claimed = if all_claimed { 1u8 } else { 0u8 };
                (all_claimed.into(), b.lockup_end.into(), pk.into())
            },
            "batch",
            "batch__promotion",
        ),
    };
    IndexedMap::new("batch", indexes)
}

pub(crate) fn save_new_batch(
    storage: &mut dyn Storage,
    batch: Batch,
    config: &Config,
) -> Result<(), ContractError> {
    let next_id = NEXT_ID.may_load(storage)?.unwrap_or_else(|| 1);
    require(
        next_id == 1 || config.is_multi_batch_enabled,
        ContractError::MultiBatchNotSupported {},
    )?;
    batches().save(storage, next_id.into(), &batch)?;
    NEXT_ID.save(storage, &(next_id + 1))?;

    Ok(())
}

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

/// Limit to batches that have not yet been promoted (0), using sub_prefix.
/// Iterate which have expired at or less than the current time (now), using a bound.
/// These are all eligible for fund claiming.
pub(crate) fn get_claimable_batch_ids(
    storage: &dyn Storage,
    current_time: u64,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> Result<Vec<U64Key>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::exclusive(U64Key::new(s)));
    // As we want to keep the last item (pk) unbounded, we increment time by 1 and use exclusive (below the next tick).
    // This ensures that we only consider batches that have started vesting.
    let max_key = (U64Key::from(current_time + 1), U64Key::from(0)).joined_key();
    let bound = Bound::Exclusive(max_key);

    let batch_ids: Vec<U64Key> = batches()
        .idx
        .claim_time
        // Only consider batches that have funds left to withdraw.
        .sub_prefix(0u8.into())
        .keys(storage, start, Some(bound), Order::Ascending)
        .take(limit)
        .map(|k| U64Key::from(k))
        .collect();

    Ok(batch_ids)
}
