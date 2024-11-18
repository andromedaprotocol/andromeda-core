use andromeda_finance::validator_staking::UnstakingTokens;
use cw_storage_plus::Item;

use cosmwasm_std::{Addr, FullDelegation};

pub const DEFAULT_VALIDATOR: Item<Addr> = Item::new("default_validator");

pub const UNSTAKING_QUEUE: Item<Vec<UnstakingTokens>> = Item::new("unstaking_queue");

pub const RESTAKING_QUEUE: Item<FullDelegation> = Item::new("restaking_queue");
