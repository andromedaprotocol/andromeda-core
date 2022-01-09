use andromeda_protocol::{
    auction::{AuctionStateResponse, Bid, ConfigResponse},
    common::OrderBy,
};
use cosmwasm_std::{Addr, StdResult, Storage, Uint128};
use cw721::Expiration;
use cw_storage_plus::{Item, Map, U128Key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp;

const MAX_LIMIT: u64 = 30;
const DEFAULT_LIMIT: u64 = 10;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub token_addr: String,
}

impl Into<ConfigResponse> for Config {
    fn into(self) -> ConfigResponse {
        ConfigResponse {
            token_addr: self.token_addr,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenAuctionState {
    pub start_time: Expiration,
    pub end_time: Expiration,
    pub high_bidder_addr: Addr,
    pub high_bidder_amount: Uint128,
    pub coin_denom: String,
    pub auction_id: Uint128,
    pub claimed: bool,
}

impl Into<AuctionStateResponse> for TokenAuctionState {
    fn into(self) -> AuctionStateResponse {
        AuctionStateResponse {
            start_time: self.start_time,
            end_time: self.end_time,
            high_bidder_addr: self.high_bidder_addr.to_string(),
            high_bidder_amount: self.high_bidder_amount,
            claimed: self.claimed,
            coin_denom: self.coin_denom,
            auction_id: self.auction_id,
        }
    }
}

pub const NEXT_AUCTION_ID: Item<Uint128> = Item::new("next_auction_id");
pub const CONFIG: Item<Config> = Item::new("config");

pub const AUCTION_IDS: Map<&str, Vec<Uint128>> = Map::new("auction_ids"); // token_id -> [auction_ids]
pub const BIDS: Map<U128Key, Vec<Bid>> = Map::new("bids"); // auction_id -> [bids]

pub const TOKEN_AUCTION_STATE: Map<U128Key, TokenAuctionState> = Map::new("auction_token_state");

pub fn read_bids<'a>(
    storage: &'a dyn Storage,
    auction_id: U128Key,
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
        let mut deps = mock_dependencies(&[]);

        BIDS.save(deps.as_mut().storage, U128Key::new(0), &get_mock_bids())
            .unwrap();

        let bids = read_bids(deps.as_ref().storage, U128Key::new(0), None, None, None).unwrap();
        assert_eq!(get_mock_bids(), bids);
    }

    #[test]
    fn read_bids_no_params_desc() {
        let mut deps = mock_dependencies(&[]);

        BIDS.save(deps.as_mut().storage, U128Key::new(0), &get_mock_bids())
            .unwrap();

        let bids = read_bids(
            deps.as_ref().storage,
            U128Key::new(0),
            None,
            None,
            Some(OrderBy::Desc),
        )
        .unwrap();
        let mut expected_bids = get_mock_bids();
        expected_bids.reverse();
        assert_eq!(expected_bids, bids);
    }

    #[test]
    fn read_bids_start_after() {
        let mut deps = mock_dependencies(&[]);

        BIDS.save(deps.as_mut().storage, U128Key::new(0), &get_mock_bids())
            .unwrap();

        let func = |order| {
            read_bids(
                deps.as_ref().storage,
                U128Key::new(0),
                Some(2),
                None,
                Some(order),
            )
            .unwrap()
        };

        let bids = func(OrderBy::Asc);
        assert_eq!(get_mock_bids()[3..], bids);

        let bids = func(OrderBy::Desc);
        let expected_bids = &mut get_mock_bids()[0..2];
        expected_bids.reverse();
        assert_eq!(expected_bids, bids);
    }

    #[test]
    fn read_bids_limit() {
        let mut deps = mock_dependencies(&[]);

        BIDS.save(deps.as_mut().storage, U128Key::new(0), &get_mock_bids())
            .unwrap();

        let func = |order| {
            read_bids(
                deps.as_ref().storage,
                U128Key::new(0),
                None,
                Some(2),
                Some(order),
            )
            .unwrap()
        };

        let bids = func(OrderBy::Asc);
        assert_eq!(get_mock_bids()[0..2], bids);

        let bids = func(OrderBy::Desc);
        let expected_bids = &mut get_mock_bids()[3..];
        expected_bids.reverse();
        assert_eq!(expected_bids, bids);
    }

    #[test]
    fn read_bids_start_after_limit() {
        let mut deps = mock_dependencies(&[]);

        BIDS.save(deps.as_mut().storage, U128Key::new(0), &get_mock_bids())
            .unwrap();

        let func = |order| {
            read_bids(
                deps.as_ref().storage,
                U128Key::new(0),
                Some(2),
                Some(1),
                Some(order),
            )
            .unwrap()
        };

        let bids = func(OrderBy::Asc);
        assert_eq!(get_mock_bids()[3..4], bids);

        let bids = func(OrderBy::Desc);
        let expected_bids = &mut get_mock_bids()[1..2];
        expected_bids.reverse();
        assert_eq!(expected_bids, bids);
    }

    #[test]
    fn read_bids_start_after_limit_too_high() {
        let mut deps = mock_dependencies(&[]);

        BIDS.save(deps.as_mut().storage, U128Key::new(0), &get_mock_bids())
            .unwrap();

        let func = |order| {
            read_bids(
                deps.as_ref().storage,
                U128Key::new(0),
                Some(2),
                Some(100),
                Some(order),
            )
            .unwrap()
        };

        let bids = func(OrderBy::Asc);
        assert_eq!(get_mock_bids()[3..], bids);

        let bids = func(OrderBy::Desc);
        let expected_bids = &mut get_mock_bids()[0..2];
        expected_bids.reverse();
        assert_eq!(expected_bids, bids);
    }

    #[test]
    fn read_bids_start_after_too_high() {
        let mut deps = mock_dependencies(&[]);

        BIDS.save(deps.as_mut().storage, U128Key::new(0), &get_mock_bids())
            .unwrap();

        let func = |order| {
            read_bids(
                deps.as_ref().storage,
                U128Key::new(0),
                Some(100),
                None,
                Some(order),
            )
            .unwrap()
        };

        let bids = func(OrderBy::Asc);
        assert!(bids.is_empty());

        let bids = func(OrderBy::Desc);
        assert!(bids.is_empty());
    }

    #[test]
    fn read_bids_start_after_and_limit_too_high() {
        let mut deps = mock_dependencies(&[]);

        BIDS.save(deps.as_mut().storage, U128Key::new(0), &get_mock_bids())
            .unwrap();

        let func = |order| {
            read_bids(
                deps.as_ref().storage,
                U128Key::new(0),
                Some(100),
                Some(100),
                Some(order),
            )
            .unwrap()
        };

        let bids = func(OrderBy::Asc);
        assert!(bids.is_empty());

        let bids = func(OrderBy::Desc);
        assert!(bids.is_empty());
    }
}
