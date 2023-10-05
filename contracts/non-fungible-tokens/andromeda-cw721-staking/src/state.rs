use andromeda_non_fungible_tokens::cw721_staking::StakedNft;

use cosmwasm_std::Coin;
use cw_storage_plus::{Item, Map};

// list of cw721 contracts that we allow NFTs from
pub const ALLOWED_CONTRACTS: Item<Vec<String>> = Item::new("allowed_contracts");

// length of unbonding period in seconds
pub const UNBONDING_PERIOD: Item<u64> = Item::new("unbonding_period");

// reward per second spent bonded, we can set a minimum staking period
pub const REWARD: Item<Coin> = Item::new("reward");

// use concatenated contract address and token id as key
pub const STAKED_NFTS: Map<String, StakedNft> = Map::new("staked_nfts");
