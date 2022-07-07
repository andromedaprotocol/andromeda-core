use andromeda_non_fungible_tokens::auction::{AuctionStateResponse, Bid};
use common::{app::AndrAddress, error::ContractError, OrderBy};
use cosmwasm_std::{Addr, Coin, Order, StdResult, Storage, Timestamp, Uint128};
use cw721::Expiration;
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp;

const MAX_LIMIT: u64 = 30;
const DEFAULT_LIMIT: u64 = 10;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StakedNft {
    pub owner: String,
    pub id: String,
    pub contract_address: String,
    pub time_of_staking: Timestamp,
    pub time_of_unbonding: Option<Timestamp>,
    pub reward: Option<Coin>,
}
// list of cw721 contracts that we allow NFTs from
pub const ALLOWED_CONTRACTS: Item<Vec<String>> = Item::new("allowed_contracts");

// length of unbonding period in seconds
pub const UNBONDING_PERIOD: Item<u64> = Item::new("unbonding_period");

// reward per second spent bonded, we can set a minimum staking period
pub const REWARD: Item<Coin> = Item::new("reward");

// use concatenated contract address and token id as key
pub const STAKED_NFTS: Map<String, StakedNft> = Map::new("staked_nfts");

pub fn read_bids(
    storage: &dyn Storage,
    auction_id: u128,
    start_after: Option<u64>,
    limit: Option<u64>,
    order_by: Option<OrderBy>,
) -> StdResult<Vec<Bid>> {
    let mut bids = BIDS.load(storage, auction_id)?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    // Passing in None implies we start from the beginning of the vector.
    let start = match start_after {
        None => 0,
        Some(x) => (x as usize) + 1usize,
    };

    // Start is INCLUSIVE, End is EXCLUSIVE
    let (start, end, order_by) = match order_by {
        Some(OrderBy::Desc) => (
            bids.len() - cmp::min(bids.len(), start + limit),
            bids.len() - cmp::min(start, bids.len()),
            OrderBy::Desc,
        ),
        // Default ordering is Ascending.
        _ => (
            cmp::min(bids.len(), start),
            cmp::min(start + limit, bids.len()),
            OrderBy::Asc,
        ),
    };

    let slice = &mut bids[start..end];
    if order_by == OrderBy::Desc {
        slice.reverse();
    }

    Ok(slice.to_vec())
}

pub fn read_auction_infos(
    storage: &dyn Storage,
    token_address: String,
    start_after: Option<String>,
    limit: Option<u64>,
) -> Result<Vec<AuctionInfo>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let keys: Vec<String> = auction_infos()
        .idx
        .token
        .prefix(token_address)
        .keys(storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<Result<Vec<String>, _>>()?;

    let mut res: Vec<AuctionInfo> = vec![];
    for key in keys.iter() {
        res.push(auction_infos().load(storage, key)?);
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::Timestamp;

    fn get_mock_bids() -> Vec<Bid> {
        vec![
            Bid {
                bidder: "0".to_string(),
                amount: Uint128::zero(),
                timestamp: Timestamp::from_seconds(0),
            },
            Bid {
                bidder: "1".to_string(),
                amount: Uint128::zero(),
                timestamp: Timestamp::from_seconds(0),
            },
            Bid {
                bidder: "2".to_string(),
                amount: Uint128::zero(),
                timestamp: Timestamp::from_seconds(0),
            },
            Bid {
                bidder: "3".to_string(),
                amount: Uint128::zero(),
                timestamp: Timestamp::from_seconds(0),
            },
            Bid {
                bidder: "4".to_string(),
                amount: Uint128::zero(),
                timestamp: Timestamp::from_seconds(0),
            },
        ]
    }

    #[test]
    fn read_bids_no_params() {
        let mut deps = mock_dependencies();

        BIDS.save(deps.as_mut().storage, 0, &get_mock_bids())
            .unwrap();

        let bids = read_bids(deps.as_ref().storage, 0, None, None, None).unwrap();
        assert_eq!(get_mock_bids(), bids);
    }

    #[test]
    fn read_bids_no_params_desc() {
        let mut deps = mock_dependencies();

        BIDS.save(deps.as_mut().storage, 0, &get_mock_bids())
            .unwrap();

        let bids = read_bids(deps.as_ref().storage, 0, None, None, Some(OrderBy::Desc)).unwrap();
        let mut expected_bids = get_mock_bids();
        expected_bids.reverse();
        assert_eq!(expected_bids, bids);
    }

    #[test]
    fn read_bids_start_after() {
        let mut deps = mock_dependencies();

        BIDS.save(deps.as_mut().storage, 0, &get_mock_bids())
            .unwrap();

        let func = |order| read_bids(deps.as_ref().storage, 0, Some(2), None, Some(order)).unwrap();

        let bids = func(OrderBy::Asc);
        assert_eq!(get_mock_bids()[3..], bids);

        let bids = func(OrderBy::Desc);
        let expected_bids = &mut get_mock_bids()[0..2];
        expected_bids.reverse();
        assert_eq!(expected_bids, bids);
    }

    #[test]
    fn read_bids_limit() {
        let mut deps = mock_dependencies();

        BIDS.save(deps.as_mut().storage, 0, &get_mock_bids())
            .unwrap();

        let func = |order| read_bids(deps.as_ref().storage, 0, None, Some(2), Some(order)).unwrap();

        let bids = func(OrderBy::Asc);
        assert_eq!(get_mock_bids()[0..2], bids);

        let bids = func(OrderBy::Desc);
        let expected_bids = &mut get_mock_bids()[3..];
        expected_bids.reverse();
        assert_eq!(expected_bids, bids);
    }

    #[test]
    fn read_bids_start_after_limit() {
        let mut deps = mock_dependencies();

        BIDS.save(deps.as_mut().storage, 0, &get_mock_bids())
            .unwrap();

        let func =
            |order| read_bids(deps.as_ref().storage, 0, Some(2), Some(1), Some(order)).unwrap();

        let bids = func(OrderBy::Asc);
        assert_eq!(get_mock_bids()[3..4], bids);

        let bids = func(OrderBy::Desc);
        let expected_bids = &mut get_mock_bids()[1..2];
        expected_bids.reverse();
        assert_eq!(expected_bids, bids);
    }

    #[test]
    fn read_bids_start_after_limit_too_high() {
        let mut deps = mock_dependencies();

        BIDS.save(deps.as_mut().storage, 0, &get_mock_bids())
            .unwrap();

        let func =
            |order| read_bids(deps.as_ref().storage, 0, Some(2), Some(100), Some(order)).unwrap();

        let bids = func(OrderBy::Asc);
        assert_eq!(get_mock_bids()[3..], bids);

        let bids = func(OrderBy::Desc);
        let expected_bids = &mut get_mock_bids()[0..2];
        expected_bids.reverse();
        assert_eq!(expected_bids, bids);
    }

    #[test]
    fn read_bids_start_after_too_high() {
        let mut deps = mock_dependencies();

        BIDS.save(deps.as_mut().storage, 0, &get_mock_bids())
            .unwrap();

        let func =
            |order| read_bids(deps.as_ref().storage, 0, Some(100), None, Some(order)).unwrap();

        let bids = func(OrderBy::Asc);
        assert!(bids.is_empty());

        let bids = func(OrderBy::Desc);
        assert!(bids.is_empty());
    }

    #[test]
    fn read_bids_start_after_and_limit_too_high() {
        let mut deps = mock_dependencies();

        BIDS.save(deps.as_mut().storage, 0, &get_mock_bids())
            .unwrap();

        let func =
            |order| read_bids(deps.as_ref().storage, 0, Some(100), Some(100), Some(order)).unwrap();

        let bids = func(OrderBy::Asc);
        assert!(bids.is_empty());

        let bids = func(OrderBy::Desc);
        assert!(bids.is_empty());
    }
}
