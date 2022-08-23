use common::{
    ado_base::recipient::Recipient, error::ContractError, require, withdraw::WithdrawalType,
};
use cosmwasm_std::{Order, Storage, Uint128};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, MultiIndex};
use cw_utils::Duration;
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
    /// The unbonding duration of the native staking module.
    pub unbonding_duration: Duration,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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
    pub last_claimed_release_time: u64,
}

// Inspired by https://docs.cosmwasm.com/tutorials/storage/indexes/#storage-plus-indexing
// We need a secondary index for batches, such that we can look up batches that
// still have funds, ordered by expiration (ascending) from now.
// Index: (U8Key/bool: batch_fully_claimed, U64Key: lockup_end) -> U64Key: pk
pub struct BatchIndexes<'a> {
    pub claim_time: MultiIndex<'a, (u8, u64), Batch, u64>,
}

impl<'a> IndexList<Batch> for BatchIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Batch>> + '_> {
        let v: Vec<&dyn Index<Batch>> = vec![&self.claim_time];
        Box::new(v.into_iter())
    }
}

pub fn batches<'a>() -> IndexedMap<'a, u64, Batch, BatchIndexes<'a>> {
    let indexes = BatchIndexes {
        claim_time: MultiIndex::new(
            |b: &Batch| {
                let all_claimed = b.amount - b.amount_claimed == Uint128::zero();
                // Allows us to skip batches that have been already fully claimed.
                let all_claimed = if all_claimed { 1u8 } else { 0u8 };
                (all_claimed, b.lockup_end)
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
    let next_id = NEXT_ID.may_load(storage)?.unwrap_or(1);
    require(
        next_id == 1 || config.is_multi_batch_enabled,
        ContractError::MultiBatchNotSupported {},
    )?;
    batches().save(storage, next_id, &batch)?;
    NEXT_ID.save(storage, &(next_id + 1))?;

    Ok(())
}

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

/// Limit to batches that have not yet been promoted (0), using sub_prefix.
/// Iterate which have expired at or less than the current time (now), using a bound.
/// These are all eligible for fund claiming.
pub(crate) fn get_claimable_batches_with_ids(
    storage: &dyn Storage,
    current_time: u64,
    limit: Option<u32>,
) -> Result<Vec<(u64, Batch)>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    // As we want to keep the last item (pk) unbounded, we increment time by 1 and use exclusive (below the next tick).
    // This ensures that we only consider batches that have started vesting.
    let max_key = (current_time + 1, 0);
    let bound = Bound::exclusive(max_key);

    let batches_with_ids: Result<Vec<(u64, Batch)>, ContractError> = batches()
        .idx
        .claim_time
        // Only consider batches that have funds left to withdraw.
        .sub_prefix(0u8)
        .range(storage, None, Some(bound), Order::Ascending)
        .take(limit)
        // Since we are iterating over a joined key and a u64 only needs 8 bytes to represent it,
        // we can obtain it like so. The need for 8 bytes comes from a byte containing 8 bits and
        // since we need 64 bits of info, we need 8 bytes (8 * 8 == 64).
        .map(|k| {
            let (k, b) = k?;

            Ok((k, b))
        })
        .collect();

    batches_with_ids
}

pub(crate) fn get_all_batches_with_ids(
    storage: &dyn Storage,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> Result<Vec<(u64, Batch)>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let batches_with_ids: Result<Vec<(u64, Batch)>, ContractError> = batches()
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|k| {
            let (k, b) = k?;
            Ok((k, b))
        })
        .collect();

    batches_with_ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};

    #[test]
    fn test_get_claimable_batches_with_ids() {
        let current_time = mock_env().block.time.seconds();

        let locked_batch = Batch {
            amount: Uint128::new(100),
            amount_claimed: Uint128::zero(),
            lockup_end: current_time + 10,
            release_unit: 10,
            release_amount: WithdrawalType::Amount(Uint128::new(10)),
            last_claimed_release_time: current_time - 1,
        };

        let unlocked_batch = Batch {
            amount: Uint128::new(100),
            amount_claimed: Uint128::zero(),
            lockup_end: current_time - 1,
            release_unit: 10,
            release_amount: WithdrawalType::Amount(Uint128::new(10)),
            last_claimed_release_time: current_time - 1,
        };

        let unlocked_but_empty_batch = Batch {
            amount: Uint128::new(100),
            amount_claimed: Uint128::new(100),
            lockup_end: current_time - 1,
            release_unit: 10,
            release_amount: WithdrawalType::Amount(Uint128::new(10)),
            last_claimed_release_time: current_time - 1,
        };

        let mut deps = mock_dependencies();

        batches()
            .save(deps.as_mut().storage, 1, &locked_batch)
            .unwrap();

        batches()
            .save(deps.as_mut().storage, 2, &unlocked_batch)
            .unwrap();

        batches()
            .save(deps.as_mut().storage, 3, &unlocked_but_empty_batch)
            .unwrap();

        let batch_ids =
            get_claimable_batches_with_ids(deps.as_ref().storage, current_time, None).unwrap();

        // Only the unlocked batch is returned since the other two are invalid in the sense of
        // withdrawing.
        assert_eq!(vec![(2, unlocked_batch)], batch_ids);
    }
}
