use andromeda_finance::vesting::Config;
use andromeda_std::{
    common::{withdraw::WithdrawalType, Milliseconds},
    error::ContractError,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Order, Storage, Uint128};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, MultiIndex};

/// The config.
pub const CONFIG: Item<Config> = Item::new("config");

/// The next ID to use for a newly added batch.
pub const NEXT_ID: Item<u64> = Item::new("next_id");

#[cw_serde]
pub struct Batch {
    /// The amount of tokens in the batch
    pub amount: Uint128,
    /// The amount of tokens that have been claimed.
    pub amount_claimed: Uint128,
    /// When the lockup ends.
    pub lockup_end: Milliseconds,
    /// How often releases occur.
    pub release_duration: Milliseconds,
    /// Specifies how much is to be released after each `release_duration`. If
    /// it is a percentage, it would be the percentage of the original amount.
    pub release_amount: WithdrawalType,
    /// The time at which the last claim took place in seconds.
    pub last_claimed_release_time: Milliseconds,
}

// Inspired by https://docs.cosmwasm.com/tutorials/storage/indexes/#storage-plus-indexing
// We need a secondary index for batches, such that we can look up batches that
// still have funds, ordered by expiration (ascending) from now.
// Index: (U8Key/bool: batch_fully_claimed, U64Key: lockup_end) -> U64Key: pk
pub struct BatchIndexes<'a> {
    pub claim_time: MultiIndex<'a, (u8, u64), Batch, u64>,
}

impl IndexList<Batch> for BatchIndexes<'_> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Batch>> + '_> {
        let v: Vec<&dyn Index<Batch>> = vec![&self.claim_time];
        Box::new(v.into_iter())
    }
}

pub fn batches() -> IndexedMap<u64, Batch, BatchIndexes<'static>> {
    let indexes = BatchIndexes {
        claim_time: MultiIndex::new(
            |_pk: &[u8], b: &Batch| {
                let all_claimed = b.amount - b.amount_claimed == Uint128::zero();
                // Allows us to skip batches that have been already fully claimed.
                let all_claimed = u8::from(all_claimed);
                (all_claimed, b.lockup_end.milliseconds())
            },
            "batch",
            "batch__promotion",
        ),
    };
    IndexedMap::new("batch", indexes)
}

pub(crate) fn save_new_batch(storage: &mut dyn Storage, batch: Batch) -> Result<(), ContractError> {
    let next_id = NEXT_ID.may_load(storage)?.unwrap_or(1);

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
    current_time: Milliseconds,
    limit: Option<u32>,
) -> Result<Vec<(u64, Batch)>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    // As we want to keep the last item (pk) unbounded, we increment time by 1 and use exclusive (below the next tick).
    // This ensures that we only consider batches that have started vesting.
    let max_key = (current_time.milliseconds() + 1, 0);
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
        let current_time = Milliseconds::from_seconds(mock_env().block.time.seconds());

        let locked_batch = Batch {
            amount: Uint128::new(100),
            amount_claimed: Uint128::zero(),
            lockup_end: current_time.plus_seconds(10),
            release_duration: Milliseconds::from_seconds(10),
            release_amount: WithdrawalType::Amount(Uint128::new(10)),
            last_claimed_release_time: current_time.minus_seconds(1),
        };

        let unlocked_batch = Batch {
            amount: Uint128::new(100),
            amount_claimed: Uint128::zero(),
            lockup_end: current_time.minus_seconds(1),
            release_duration: Milliseconds::from_seconds(10),
            release_amount: WithdrawalType::Amount(Uint128::new(10)),
            last_claimed_release_time: current_time.minus_seconds(1),
        };

        let unlocked_but_empty_batch = Batch {
            amount: Uint128::new(100),
            amount_claimed: Uint128::new(100),
            lockup_end: current_time.minus_seconds(1),
            release_duration: Milliseconds::from_seconds(10),
            release_amount: WithdrawalType::Amount(Uint128::new(10)),
            last_claimed_release_time: current_time.minus_seconds(1),
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
