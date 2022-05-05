use common::{ado_base::recipient::Recipient, error::ContractError, withdraw::WithdrawalType};
use cosmwasm_std::{Storage, Uint128};
use cw0::{Duration, Expiration};
use cw_storage_plus::{Item, Map};
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
    /// When the lockup ends. None indicates no lock up period and funds start vesting right away.
    pub lockup_end: Option<Expiration>,
    /// How often releases occur.
    pub release_unit: Duration,
    /// Specifies how much is to be released after each `release_unit`. If
    /// it is a percentage, it would be the percentage of the original amount.
    pub release_amount: WithdrawalType,
    /// The time at which the last claim took place. Either height or seconds.
    pub last_claim_time: Expiration,
}

pub(crate) fn save_batch(storage: &mut dyn Storage, batch: Batch) -> Result<(), ContractError> {
    let next_id = NEXT_ID
        .may_load(storage)?
        .unwrap_or_else(|| Uint128::new(1));

    BATCHES.save(storage, &next_id.to_string(), &batch)?;
    NEXT_ID.save(storage, &(next_id + Uint128::new(1)))?;

    Ok(())
}
